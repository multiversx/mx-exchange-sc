multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait AdminActionsModule:
    super::user_deposit_withdraw::UserDepositWithdrawModule
    + crate::common_storage::CommonStorageModule
    + crate::events::EventsModule
    + crate::phase::PhaseModule
    + crate::redeem_token::RedeemTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[endpoint(setMinLaunchedTokens)]
    fn set_min_launched_tokens(&self, min_launched_tokens: BigUint) {
        self.require_caller_admin();
        require!(min_launched_tokens > 0, "Invalid min launched tokens");

        self.min_launched_tokens().set(min_launched_tokens);
    }

    /// `whitelist` arguments are pairs of (address, max_total_deposit). Pass `0` for `max_total_deposit` if there is no limit
    #[endpoint(addUsersToWhitelist)]
    fn add_users_to_whitelist(
        &self,
        whitelist: MultiValueEncoded<MultiValue2<ManagedAddress, BigUint>>,
    ) {
        self.require_caller_admin();

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
    }

    #[endpoint(refundUsers)]
    fn refund_users(&self, users: MultiValueEncoded<ManagedAddress>) {
        self.require_caller_admin();

        let id_mapper = self.id_mapper();
        let whitelist_mapper = self.user_whitelist();
        let owner_address = self.blockchain().get_owner_address();
        let mut redeem_token_supply = self.redeem_token_total_circulating_supply().get();
        for user in users {
            require!(user != owner_address, "May not refund owner");

            let user_id = id_mapper.get_id_non_zero(&user);
            whitelist_mapper.require_whitelisted(&user_id);
            whitelist_mapper.remove(&user_id);

            let user_deposit = self.total_user_deposit(user_id).take();
            self.user_deposit_limit(user_id).clear();

            if user_deposit == 0 {
                continue;
            }

            let accepted_token_id = self.accepted_token_id().get();
            self.send()
                .direct(&user, &accepted_token_id, 0, &user_deposit);
            redeem_token_supply -= user_deposit;
        }

        self.redeem_token_total_circulating_supply()
            .set(redeem_token_supply);
    }

    fn require_caller_admin(&self) {
        let caller = self.blockchain().get_caller();
        let admin = self.admin().get();
        require!(caller == admin, "Only admin may call this function");
    }

    #[storage_mapper("admin")]
    fn admin(&self) -> SingleValueMapper<ManagedAddress>;
}
