use common_types::PaymentsVec;
use router_proxy::SwapOperationType;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod router_proxy {
    use common_types::PaymentsVec;

    multiversx_sc::imports!();

    pub type FunctionName<M> = ManagedBuffer<M>;
    pub type SwapOperationType<M> =
        MultiValue4<ManagedAddress<M>, FunctionName<M>, TokenIdentifier<M>, BigUint<M>>;

    #[multiversx_sc::proxy]
    pub trait RouterProxy {
        #[payable("*")]
        #[endpoint(multiPairSwap)]
        fn multi_pair_swap(
            &self,
            swap_operations: MultiValueEncoded<SwapOperationType<Self::Api>>,
        ) -> PaymentsVec<Self::Api>;
    }
}

#[multiversx_sc::module]
pub trait RouterInteractionsModule:
    crate::fees_accumulation::FeesAccumulationModule
    + crate::config::ConfigModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
    + multiversx_sc_modules::only_admin::OnlyAdminModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
{
    #[only_owner]
    #[endpoint(setRouterAddress)]
    fn set_router_address(&self, router_address: ManagedAddress) {
        self.require_sc_address(&router_address);

        self.router_address().set(router_address);
    }

    /// Swaps tokens to the base token (i.e. MEX).
    ///
    /// `token_to_send` must be a known token to the fees collector, and the very last token received must be MEX.
    ///
    /// The fees collector uses the given pair paths through the router contract.
    ///
    /// `swap_operations` are pairs of (pair address, pair function name, token wanted, min amount out)
    ///
    /// "pair function name" can only be "swapTokensFixedInput" or "swapTokensFixedOutput"
    ///
    /// "min amount out" is a minimum of 1
    #[only_admin]
    #[endpoint(swapTokenToBaseToken)]
    fn swap_token_to_base_token(
        &self,
        token_to_send: EsdtTokenPayment,
        swap_operations: MultiValueEncoded<SwapOperationType<Self::Api>>,
    ) {
        self.check_swap_through_router_args(&token_to_send, &swap_operations);
        let current_week = self.get_current_week();

        let token_max_amount =
            self.get_token_available_amount(current_week, &token_to_send.token_identifier);
        require!(token_max_amount > 0, "No tokens available for swap");
        require!(
            token_to_send.amount <= token_max_amount,
            "Not enough tokens available for swap"
        );
        let token_amount = token_to_send.amount;

        let router_address = self.router_address().get();
        let swap_payment = EsdtTokenPayment::new(token_to_send.token_identifier, 0, token_amount);
        let mut received_tokens =
            self.call_swap_through_router(router_address.clone(), swap_payment, swap_operations);

        let base_token_id = self.get_base_token_id();
        require!(
            received_tokens.token_identifier == base_token_id,
            "Invalid tokens received from router"
        );

        self.burn_part_of_base_token(&mut received_tokens);

        self.accumulated_fees(current_week, &base_token_id)
            .update(|acc_fees| *acc_fees += received_tokens.amount);
    }

    fn check_swap_through_router_args(
        &self,
        token_to_send: &EsdtTokenPayment,
        swap_operation: &MultiValueEncoded<SwapOperationType<Self::Api>>,
    ) {
        require!(!swap_operation.is_empty(), "No arguments provided");

        let base_token_id = self.get_base_token_id();
        let locked_token_id = self.get_locked_token_id();
        require!(
            token_to_send.token_identifier != base_token_id
                && token_to_send.token_identifier != locked_token_id,
            "May not swap base token or locked token"
        );
        require!(
            token_to_send.amount > 0,
            "Token amount to swap must be greater than zero"
        );
    }

    fn call_swap_through_router(
        &self,
        router_address: ManagedAddress,
        payment: EsdtTokenPayment,
        swap_operations: MultiValueEncoded<SwapOperationType<Self::Api>>,
    ) -> EsdtTokenPayment {
        let output_payments: PaymentsVec<Self::Api> = self
            .router_proxy(router_address)
            .multi_pair_swap(swap_operations)
            .esdt(payment)
            .execute_on_dest_context();
        require!(
            !output_payments.is_empty(),
            "No payments received from router"
        );

        unsafe { output_payments.iter().next_back().unwrap_unchecked() }
    }

    #[storage_mapper("routerAddress")]
    fn router_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[proxy]
    fn router_proxy(&self, sc_address: ManagedAddress) -> router_proxy::Proxy<Self::Api>;
}
