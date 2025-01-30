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
    /// Pass `whitelist_complete` as `true` if these are the last entries
    ///
    /// This ensures the new owner can't add additional addresses after the setup phase
    ///
    /// `whitelist` arguments are pairs of (address, max_total_deposit). Pass `0` for `max_total_deposit` if there is no limit
    #[only_owner]
    #[endpoint(addUsersToWhitelist)]
    fn add_users_to_whitelist(
        &self,
        whitelist_complete: bool,
        whitelist: MultiValueEncoded<MultiValue2<ManagedAddress, BigUint>>,
    ) {
        let whitelist_complete_mapper = self.whitelist_complete();
        require!(
            !whitelist_complete_mapper.get(),
            "Whitelist already complete"
        );

        let id_mapper = self.id_mapper();
        let whitelist_mapper = self.user_whitelist();
        for pair in whitelist {
            let (user, limit) = pair.into_tuple();
            let user_id = id_mapper.insert_new(&user);
            whitelist_mapper.add(&user_id);

            if limit > 0 {
                self.user_deposit_limit(user_id).set(limit);
            }
        }

        if whitelist_complete {
            whitelist_complete_mapper.set(true);
        }
    }

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
        let accepted_token_id = self.accepted_token_id().get();
        require!(payment_token == accepted_token_id, INVALID_PAYMENT_ERR_MSG);

        self.add_and_require_user_deposit_under_limit(user_id, &payment_amount);

        self.accepted_token_balance()
            .update(|balance| *balance += &payment_amount);

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
        self.require_redeem_token_setup_complete();

        let phase = self.get_current_phase();
        self.require_user_deposit_withdraw_allowed(&phase);

        let caller = self.blockchain().get_caller();
        let user_id = self.require_user_whitelisted(&caller);

        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        let redeem_token_id = self.redeem_token().get_token_id();
        require!(payment_token == redeem_token_id, INVALID_PAYMENT_ERR_MSG);

        self.total_user_deposit(user_id).update(|total_deposit| {
            *total_deposit -= &payment_amount;
        });

        self.burn_redeem_token(&payment_amount);
        self.accepted_token_balance()
            .update(|balance| *balance -= &payment_amount);

        let refund_token_id = self.accepted_token_id().get();
        self.send()
            .direct(&caller, &refund_token_id, 0, &payment_amount);

        self.emit_user_withdraw_event(&payment_amount, &payment_token, &payment_amount);

        EgldOrEsdtTokenPayment::new(refund_token_id, 0, payment_amount)
    }

    #[view(isUserWhitelisted)]
    fn is_user_whitelisted(&self, user: ManagedAddress) -> bool {
        let user_id = self.id_mapper().get_id(&user);
        if user_id != NULL_ID {
            self.user_whitelist().contains(&user_id)
        } else {
            false
        }
    }

    fn require_user_whitelisted(&self, user: &ManagedAddress) -> AddressId {
        let user_id = self.id_mapper().get_id(user);
        require!(
            user_id != NULL_ID && self.user_whitelist().contains(&user_id),
            "User not whitelisted"
        );

        user_id
    }

    fn add_and_require_user_deposit_under_limit(&self, user_id: AddressId, user_deposit: &BigUint) {
        let limit = self.user_deposit_limit(user_id).get();
        let total_deposit = self.total_user_deposit(user_id).update(|total_deposit| {
            *total_deposit += user_deposit;

            (*total_deposit).clone()
        });

        if limit == 0 {
            return;
        }

        require!(total_deposit <= limit, "Exceeded deposit limit");
    }

    #[storage_mapper("idMapper")]
    fn id_mapper(&self) -> AddressToIdMapper;

    #[storage_mapper("userWhitelist")]
    fn user_whitelist(&self) -> WhitelistMapper<AddressId>;

    #[storage_mapper("whitelistComplete")]
    fn whitelist_complete(&self) -> SingleValueMapper<bool>;

    #[storage_mapper("userDepositLimit")]
    fn user_deposit_limit(&self, user_id: AddressId) -> SingleValueMapper<BigUint>;

    #[storage_mapper("totalUserDeposit")]
    fn total_user_deposit(&self, user_id: AddressId) -> SingleValueMapper<BigUint>;
}
