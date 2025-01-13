multiversx_sc::imports!();

pub static INVALID_PAYMENT_ERR_MSG: &[u8] = b"Invalid payment token";

#[multiversx_sc::module]
pub trait UserDepositWithdrawModule:
    crate::common_storage::CommonStorageModule
    + crate::events::EventsModule
    + crate::phase::PhaseModule
    + crate::redeem_token::RedeemTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    /// Users can deposit accepted_tokens.
    /// They will receive an ESDT that can be used to withdraw launched tokens
    #[payable("*")]
    #[endpoint(userDeposit)]
    fn user_deposit(&self) -> EsdtTokenPayment {
        let phase = self.get_current_phase();
        self.require_user_deposit_withdraw_allowed(&phase);

        let (payment_token, payment_amount) = self.call_value().egld_or_single_fungible_esdt();
        let accepted_token_id = self.accepted_token_id().get();
        require!(payment_token == accepted_token_id, INVALID_PAYMENT_ERR_MSG);

        self.accepted_token_balance()
            .update(|balance| *balance += &payment_amount);

        let caller = self.blockchain().get_caller();
        let payment_result = self.mint_and_send_redeem_token(&caller, payment_amount.clone());

        self.emit_user_deposit_event(
            &payment_amount,
            &payment_result.token_identifier,
            &payment_amount,
        );

        payment_result
    }

    /// Deposit ESDT received after deposit to withdraw the initially deposited tokens.
    #[payable("*")]
    #[endpoint(userWithdraw)]
    fn user_withdraw(&self) -> EgldOrEsdtTokenPayment {
        let phase = self.get_current_phase();
        self.require_user_deposit_withdraw_allowed(&phase);

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        let redeem_token_id = self.redeem_token().get_token_id();
        require!(payment_token == redeem_token_id, INVALID_PAYMENT_ERR_MSG);

        self.burn_redeem_token(&payment_amount);
        self.accepted_token_balance()
            .update(|balance| *balance -= &payment_amount);

        let refund_token_id = self.accepted_token_id().get();

        let caller = self.blockchain().get_caller();
        self.send()
            .direct(&caller, &refund_token_id, 0, &payment_amount);

        self.emit_user_withdraw_event(&payment_amount, &payment_token, &payment_amount);

        EgldOrEsdtTokenPayment::new(refund_token_id, 0, payment_amount)
    }
}
