use crate::user_actions::user_deposit_withdraw::INVALID_PAYMENT_ERR_MSG;

multiversx_sc::imports!();

pub static INVALID_AMOUNT_ERR_MSG: &[u8] = b"Invalid amount";

#[multiversx_sc::module]
pub trait OwnerDepositWithdrawModule:
    crate::common_storage::CommonStorageModule
    + crate::events::EventsModule
    + crate::phase::PhaseModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[only_owner]
    #[payable("*")]
    #[endpoint(ownerDeposit)]
    fn owner_deposit(&self) {
        let min_launched_tokens = self.min_launched_tokens().get();
        require!(min_launched_tokens > 0, "Min launched tokens not set yet");

        let phase = self.get_current_phase();
        self.require_owner_deposit_withdraw_allowed(&phase);

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        let launched_token_id = self.launched_token_id().get();
        require!(payment_token == launched_token_id, INVALID_PAYMENT_ERR_MSG);

        let current_total_launched_tokens_amount = self.launched_token_balance().get();
        let new_total = &current_total_launched_tokens_amount + &payment_amount;
        require!(new_total >= min_launched_tokens, INVALID_AMOUNT_ERR_MSG);

        self.launched_token_balance().set(new_total);

        self.emit_owner_deposit_event(&payment_amount);
    }

    #[only_owner]
    #[endpoint(ownerWithdraw)]
    fn owner_withdraw(&self, withdraw_amount: BigUint) -> EsdtTokenPayment {
        let phase = self.get_current_phase();
        self.require_owner_deposit_withdraw_allowed(&phase);

        let current_total_launched_tokens = self.launched_token_balance().get();
        require!(
            withdraw_amount > 0 && withdraw_amount <= current_total_launched_tokens,
            INVALID_AMOUNT_ERR_MSG
        );

        let min_launched_tokens = self.min_launched_tokens().get();
        require!(
            &current_total_launched_tokens - &withdraw_amount >= min_launched_tokens,
            INVALID_AMOUNT_ERR_MSG
        );

        self.launched_token_balance()
            .update(|balance| *balance -= &withdraw_amount);

        let launched_token_id = self.launched_token_id().get();
        let caller = self.blockchain().get_caller();
        self.send()
            .direct_esdt(&caller, &launched_token_id, 0, &withdraw_amount);

        self.emit_owner_withdraw_event(&withdraw_amount);

        EsdtTokenPayment::new(launched_token_id, 0, withdraw_amount)
    }
}
