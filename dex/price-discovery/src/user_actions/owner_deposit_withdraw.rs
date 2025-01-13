use crate::user_actions::user_deposit_withdraw::INVALID_PAYMENT_ERR_MSG;

multiversx_sc::imports!();

pub static INVALID_AMOUNT_ERR_MSG: &[u8] = b"Invalid amount";

#[multiversx_sc::module]
pub trait OwnerDepositWithdrawModule:
    crate::common_storage::CommonStorageModule
    + crate::events::EventsModule
    + crate::phase::PhaseModule
    + crate::redeem_token::RedeemTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[payable("*")]
    #[endpoint(ownerDeposit)]
    fn owner_deposit(&self) {
        let phase = self.get_current_phase();
        self.require_owner_deposit_withdraw_allowed(&phase);

        let caller = self.blockchain().get_caller();
        self.require_owner_caller(&caller);

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        let launched_token_id = self.launched_token_id().get();
        require!(payment_token == launched_token_id, INVALID_PAYMENT_ERR_MSG);

        let min_launched_tokens = self.min_launched_tokens().get();
        let current_total_launched_tokens = self.launched_token_balance().get();
        require!(
            &current_total_launched_tokens + &payment_amount >= min_launched_tokens,
            INVALID_AMOUNT_ERR_MSG
        );

        self.launched_token_balance()
            .update(|balance| *balance += payment_amount);
    }

    #[endpoint(ownerWithdraw)]
    fn owner_withdraw(&self, amount: BigUint) -> EsdtTokenPayment {
        let phase = self.get_current_phase();
        self.require_owner_deposit_withdraw_allowed(&phase);

        let caller = self.blockchain().get_caller();
        self.require_owner_caller(&caller);

        let current_total_launched_tokens = self.launched_token_balance().get();
        require!(
            amount > 0 && amount <= current_total_launched_tokens,
            INVALID_AMOUNT_ERR_MSG
        );

        let min_launched_tokens = self.min_launched_tokens().get();
        require!(
            &current_total_launched_tokens - &amount >= min_launched_tokens,
            INVALID_AMOUNT_ERR_MSG
        );

        self.launched_token_balance()
            .update(|balance| *balance -= &amount);

        let launched_token_id = self.launched_token_id().get();
        self.send()
            .direct_esdt(&caller, &launched_token_id, 0, &amount);

        EsdtTokenPayment::new(launched_token_id, 0, amount)
    }

    fn require_owner_caller(&self, caller: &ManagedAddress) {
        let owner = self.owner_address().get();
        require!(caller == &owner, "Only owner may call this endpoint");
    }
}
