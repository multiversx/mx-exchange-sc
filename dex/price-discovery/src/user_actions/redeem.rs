use crate::user_actions::user_deposit_withdraw::INVALID_PAYMENT_ERR_MSG;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait RedeemModule:
    crate::common_storage::CommonStorageModule
    + crate::events::EventsModule
    + crate::phase::PhaseModule
    + crate::redeem_token::RedeemTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    /// After all phases have ended,
    /// users can withdraw their fair share of launched tokens,
    /// while the owner can withdraw the accepted tokens.
    #[payable("*")]
    #[endpoint]
    fn redeem(&self) -> EgldOrEsdtTokenPayment {
        let phase = self.get_current_phase();
        self.require_redeem_allowed(&phase);

        let caller = self.blockchain().get_caller();
        let owner = self.owner_address().get();
        if caller == owner {
            let redeemed_tokens = self.owner_redeem(&caller);

            self.emit_redeem_event(
                None,
                &BigUint::zero(),
                &redeemed_tokens.token_identifier,
                &redeemed_tokens.amount,
            );

            return redeemed_tokens;
        }

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        let bought_tokens = self.user_redeem(&caller, &payment_token, &payment_amount);

        self.emit_redeem_event(
            Some(&payment_token),
            &payment_amount,
            &bought_tokens.token_identifier,
            &bought_tokens.amount,
        );

        bought_tokens
    }

    fn owner_redeem(&self, owner: &ManagedAddress) -> EgldOrEsdtTokenPayment {
        let launched_token_supply = self.launched_token_balance().get();
        require!(
            launched_token_supply > 0,
            "May not withdraw tokens as launched tokens were not deposited"
        );

        let accepted_token_id = self.accepted_token_id().get();
        let accepted_token_balance = self.accepted_token_balance().get();
        self.send()
            .direct(owner, &accepted_token_id, 0, &accepted_token_balance);

        EgldOrEsdtTokenPayment::new(accepted_token_id, 0, accepted_token_balance)
    }

    fn user_redeem(
        &self,
        user: &ManagedAddress,
        payment_token: &TokenIdentifier,
        payment_amount: &BigUint,
    ) -> EgldOrEsdtTokenPayment {
        let redeem_token_id = self.redeem_token().get_token_id();
        require!(payment_token == &redeem_token_id, INVALID_PAYMENT_ERR_MSG);

        let launched_token_supply = self.launched_token_balance().get();
        if launched_token_supply == 0 {
            let accepted_token_id = self.accepted_token_id().get();
            self.send()
                .direct(user, &accepted_token_id, 0, payment_amount);

            return EgldOrEsdtTokenPayment::new(accepted_token_id, 0, payment_amount.clone());
        }

        let bought_tokens = self.compute_user_bought_tokens(payment_amount);
        self.burn_redeem_token_without_supply_decrease(payment_amount);

        self.send().direct(
            user,
            &bought_tokens.token_identifier,
            0,
            &bought_tokens.amount,
        );

        bought_tokens
    }

    fn compute_user_bought_tokens(&self, redeem_token_amount: &BigUint) -> EgldOrEsdtTokenPayment {
        let redeem_token_supply = self.redeem_token_total_circulating_supply().get();
        let launched_token_id = EgldOrEsdtTokenIdentifier::esdt(self.launched_token_id().get());
        let total_token_supply = self.launched_token_balance().get();
        let reward_amount = total_token_supply * redeem_token_amount / redeem_token_supply;

        EgldOrEsdtTokenPayment::new(launched_token_id, 0, reward_amount)
    }
}
