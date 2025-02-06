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
        self.require_redeem_token_setup_complete();

        let phase = self.get_current_phase();
        self.require_user_deposit_withdraw_allowed(&phase);

        let caller = self.blockchain().get_caller();
        let user_id = self.require_user_whitelisted(&caller);
        let (payment_token, payment_amount) = self.call_value().egld_or_single_fungible_esdt();
        self.add_user_deposit(user_id, &payment_token, &payment_amount);

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
    fn user_withdraw_endpoint(&self) -> EgldOrEsdtTokenPayment {
        self.require_redeem_token_setup_complete();

        let phase = self.get_current_phase();
        self.require_user_deposit_withdraw_allowed(&phase);

        let caller = self.blockchain().get_caller();
        let user_id = self.require_user_whitelisted(&caller);
        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();

        let redeem_token_id = self.redeem_token().get_token_id();
        require!(payment_token == redeem_token_id, INVALID_PAYMENT_ERR_MSG);

        self.user_withdraw(&caller, user_id, &payment_amount);

        self.burn_redeem_token(&payment_amount);

        self.emit_user_withdraw_event(&payment_amount, &payment_token, &payment_amount);

        let refund_token_id = self.accepted_token_id().get();
        EgldOrEsdtTokenPayment::new(refund_token_id, 0, payment_amount)
    }

    #[view(isUserWhitelisted)]
    fn is_user_whitelisted(&self, user: &ManagedAddress) -> bool {
        let user_id = self.user_id_mapper().get_id(user);
        if user_id != NULL_ID {
            self.user_whitelist().contains(&user_id)
        } else {
            false
        }
    }

    #[view(getUserDepositLimit)]
    fn get_user_deposit_limit(&self, user: ManagedAddress) -> OptionalValue<BigUint> {
        let user_id = self.user_id_mapper().get_id(&user);
        if user_id == NULL_ID {
            return OptionalValue::None;
        }

        let user_deposit_limit = self.user_deposit_limit(user_id).get();
        if user_deposit_limit == 0 {
            OptionalValue::None
        } else {
            OptionalValue::Some(user_deposit_limit)
        }
    }

    /// Returns the user ID
    fn require_user_whitelisted(&self, user: &ManagedAddress) -> AddressId {
        let user_id = self.user_id_mapper().get_id(user);
        require!(
            user_id != NULL_ID && self.user_whitelist().contains(&user_id),
            "User not whitelisted"
        );

        user_id
    }

    fn add_user_deposit(
        &self,
        user_id: AddressId,
        payment_token: &EgldOrEsdtTokenIdentifier,
        payment_amount: &BigUint,
    ) {
        let accepted_token_id = self.accepted_token_id().get();
        require!(payment_token == &accepted_token_id, INVALID_PAYMENT_ERR_MSG);

        self.add_and_require_valid_deposit_amount(user_id, payment_amount);

        self.accepted_token_balance()
            .update(|balance| *balance += payment_amount);
    }

    fn user_withdraw(&self, caller: &ManagedAddress, user_id: AddressId, payment_amount: &BigUint) {
        self.total_user_deposit(user_id).update(|total_deposit| {
            *total_deposit -= payment_amount;

            if *total_deposit == 0 {
                return;
            }

            let min_deposit = self.user_min_deposit().get();
            require!(*total_deposit >= min_deposit, "Withdrawing too many tokens");
        });

        self.accepted_token_balance()
            .update(|balance| *balance -= payment_amount);

        let refund_token_id = self.accepted_token_id().get();
        self.send()
            .direct(caller, &refund_token_id, 0, payment_amount);
    }

    fn add_and_require_valid_deposit_amount(&self, user_id: AddressId, user_deposit: &BigUint) {
        self.total_user_deposit(user_id).update(|total_deposit| {
            *total_deposit += user_deposit;

            let min_deposit = self.user_min_deposit().get();
            require!(*total_deposit >= min_deposit, "Not enough tokens deposited");

            let limit = self.user_deposit_limit(user_id).get();
            if limit > 0 {
                require!(*total_deposit <= limit, "Exceeded deposit limit");
            }
        });
    }

    #[storage_mapper("userIdMapper")]
    fn user_id_mapper(&self) -> AddressToIdMapper;

    #[storage_mapper("userWhitelist")]
    fn user_whitelist(&self) -> WhitelistMapper<AddressId>;

    #[storage_mapper("userMinDeposit")]
    fn user_min_deposit(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("userDepositLimit")]
    fn user_deposit_limit(&self, user_id: AddressId) -> SingleValueMapper<BigUint>;

    #[storage_mapper("totalUserDeposit")]
    fn total_user_deposit(&self, user_id: AddressId) -> SingleValueMapper<BigUint>;
}
