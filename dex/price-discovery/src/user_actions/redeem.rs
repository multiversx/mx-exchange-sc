multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait RedeemModule:
    super::user_deposit_withdraw::UserDepositWithdrawModule
    + crate::common_storage::CommonStorageModule
    + crate::events::EventsModule
    + crate::phase::PhaseModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    /// After all phases have ended,
    /// users can withdraw their fair share of launched tokens,
    /// while the owner can withdraw the accepted tokens.
    #[endpoint]
    fn redeem(&self) -> EgldOrEsdtTokenPayment {
        let phase = self.get_current_phase();
        self.require_redeem_allowed(&phase);

        let caller = self.blockchain().get_caller();
        let owner = self.blockchain().get_owner_address();
        if caller != owner {
            let bought_tokens = self.user_redeem(&caller);
            self.emit_redeem_event(&bought_tokens.token_identifier, &bought_tokens.amount);

            bought_tokens
        } else {
            let redeemed_tokens = self.owner_redeem(&caller);
            self.emit_redeem_event(&redeemed_tokens.token_identifier, &redeemed_tokens.amount);

            redeemed_tokens
        }
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

    fn user_redeem(&self, user: &ManagedAddress) -> EgldOrEsdtTokenPayment {
        let user_id = self.require_user_whitelisted(user);
        let total_user_deposit = self.total_deposit_by_user(user_id).take();

        let launched_token_supply = self.launched_token_balance().get();
        if launched_token_supply != 0 {
            let bought_tokens = self.compute_user_bought_tokens(&total_user_deposit);
            self.send().direct_non_zero(
                user,
                &bought_tokens.token_identifier,
                0,
                &bought_tokens.amount,
            );

            bought_tokens
        } else {
            let accepted_token_id = self.accepted_token_id().get();
            self.send()
                .direct_non_zero(user, &accepted_token_id, 0, &total_user_deposit);

            EgldOrEsdtTokenPayment::new(accepted_token_id, 0, total_user_deposit)
        }
    }

    fn compute_user_bought_tokens(&self, redeem_amount: &BigUint) -> EgldOrEsdtTokenPayment {
        let total_deposit_all_users = self.accepted_token_balance().get();
        let launched_token_id = EgldOrEsdtTokenIdentifier::esdt(self.launched_token_id().get());
        let total_launched_token_supply = self.launched_token_balance().get();
        let reward_amount = total_launched_token_supply * redeem_amount / total_deposit_all_users;

        EgldOrEsdtTokenPayment::new(launched_token_id, 0, reward_amount)
    }
}
