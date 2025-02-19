use crate::phase::Phase;

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

        let phase = self.get_current_phase();
        require!(
            !matches!(phase, Phase::Redeem),
            "May not set min launched tokens during redeem phase"
        );

        self.min_launched_tokens().set(min_launched_tokens);
    }

    /// Pass `0` for `limit` if there is no limit
    #[endpoint(setUserLimit)]
    fn set_user_limit(&self, user: ManagedAddress, limit: BigUint) {
        self.require_caller_admin();

        let user_id = self.user_id_mapper().get_id_non_zero(&user);
        let user_current_deposit = self.total_user_deposit(user_id).get();
        if user_current_deposit == 0 || limit == 0 {
            self.set_user_deposit_limit(&user, user_id, &limit);

            return;
        }

        require!(
            user_current_deposit <= limit,
            "May not set user limit below current deposit value"
        );

        self.set_user_deposit_limit(&user, user_id, &limit);
    }

    /// `whitelist` arguments are pairs of (address, max_total_deposit). Pass `0` for `max_total_deposit` if there is no limit
    #[endpoint(addUsersToWhitelist)]
    fn add_users_to_whitelist(
        &self,
        whitelist: MultiValueEncoded<MultiValue2<ManagedAddress, BigUint>>,
    ) {
        self.require_caller_admin();

        let phase = self.get_current_phase();
        require!(
            !matches!(phase, Phase::Redeem),
            "May not add new users during redeem phase"
        );

        let id_mapper = self.user_id_mapper();
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

        let phase = self.get_current_phase();
        require!(
            !matches!(phase, Phase::Redeem),
            "May not refund user during redeem phase"
        );

        let id_mapper = self.user_id_mapper();
        let whitelist_mapper = self.user_whitelist();
        let owner_address = self.blockchain().get_owner_address();
        let mut redeem_token_supply = self.redeem_token_total_circulating_supply().get();
        for user in users {
            require!(user != owner_address, "May not refund owner");

            let user_id = id_mapper.get_id_non_zero(&user);
            whitelist_mapper.require_whitelisted(&user_id);
            whitelist_mapper.remove(&user_id);

            let user_deposit = self.total_user_deposit(user_id).get();
            self.user_deposit_limit(user_id).clear();
            self.user_withdraw(&user, user_id, &user_deposit);

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

    fn set_user_deposit_limit(
        &self,
        user_addr: &ManagedAddress,
        user_id: AddressId,
        limit: &BigUint,
    ) {
        self.user_deposit_limit(user_id).set(limit);
        self.set_user_limit_event(user_addr, limit);
    }

    #[storage_mapper("admin")]
    fn admin(&self) -> SingleValueMapper<ManagedAddress>;
}
