elrond_wasm::imports!();

mod liquidity_pool_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait LiquidityPool {
        #[payable("*")]
        #[endpoint(addInitialLiquidity)]
        fn add_initial_liquidity(
            &self,
            #[payment_multi] payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
            #[var_args] opt_accept_funds_func: OptionalValue<ManagedBuffer>,
        ) -> MultiValue3<
            EsdtTokenPayment<Self::Api>,
            EsdtTokenPayment<Self::Api>,
            EsdtTokenPayment<Self::Api>,
        >;
    }
}

#[elrond_wasm::module]
pub trait CreatePoolModule:
    crate::common_storage::CommonStorageModule + crate::events::EventsModule
{
    #[only_owner]
    #[endpoint(setPairAddress)]
    fn set_pair_address(&self, dex_sc_address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&dex_sc_address),
            "Invalid DEX SC address"
        );
        self.dex_sc_address().set(&dex_sc_address);
    }

    /// Can only be called during unbond period, i.e. last phase.
    /// Sends all accepted and launched tokens to the liquidity pool,
    /// receiving LP tokens, which are locked for a number of epochs
    /// before redeeming is allowed
    #[endpoint(createDexLiquidityPool)]
    fn create_dex_liquidity_pool(&self) {
        require!(!self.dex_sc_address().is_empty(), "Pair address not set");
        require!(self.lp_token_id().is_empty(), "Pool already created");
        self.require_deposit_period_ended();

        let launched_token_id = self.launched_token_id().get();
        let accepted_token_id = self.accepted_token_id().get();

        let launched_token_balance = self.launched_token_balance().get();
        let accepted_token_balance = self.accepted_token_balance().get();
        let extra_rewards_balance = self.extra_rewards_balance().get();

        self.launched_token_balance().clear();
        self.accepted_token_balance().clear();
        self.total_extra_rewards_tokens()
            .set(&extra_rewards_balance);

        require!(
            launched_token_balance > 0,
            "No Launched tokens were deposited"
        );
        require!(accepted_token_balance > 0, "No users deposited tokens");

        let mut payments = ManagedVec::<Self::Api, EsdtTokenPayment<Self::Api>>::new();
        payments.push(EsdtTokenPayment {
            token_type: EsdtTokenType::Fungible,
            token_identifier: launched_token_id,
            token_nonce: 0,
            amount: launched_token_balance,
        });
        payments.push(EsdtTokenPayment {
            token_type: EsdtTokenType::Fungible,
            token_identifier: accepted_token_id,
            token_nonce: 0,
            amount: accepted_token_balance,
        });

        let dex_sc_address = self.dex_sc_address().get();
        let contract_call = self
            .dex_proxy(dex_sc_address)
            .add_initial_liquidity(payments, OptionalValue::None);

        let (lp_token, _, _) = contract_call
            .execute_on_dest_context_custom_range(|_, after| (after - 3, after))
            .into_tuple();

        self.lp_token_id().set(&lp_token.token_identifier);
        self.total_lp_tokens_received().set(&lp_token.amount);

        let current_epoch = self.blockchain().get_block_epoch();
        self.pool_creation_epoch().set(&current_epoch);

        self.emit_initial_liquidity_event(lp_token.token_identifier, lp_token.amount);
    }

    // private

    fn require_deposit_period_ended(&self) {
        let current_block = self.blockchain().get_block_nonce();
        let end_block = self.end_block().get();
        require!(current_block >= end_block, "Deposit period has not ended");
    }

    fn require_dex_address_set(&self) {
        require!(!self.dex_sc_address().is_empty(), "Pair address not set");
    }

    #[proxy]
    fn dex_proxy(&self, sc_address: ManagedAddress) -> liquidity_pool_proxy::Proxy<Self::Api>;

    #[view(getDexScAddress)]
    #[storage_mapper("dexScAddress")]
    fn dex_sc_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("poolCreationEpoch")]
    fn pool_creation_epoch(&self) -> SingleValueMapper<u64>;

    #[view(getUnbondPeriodEpochs)]
    #[storage_mapper("unbondPeriodEpochs")]
    fn unbond_period_epochs(&self) -> SingleValueMapper<u64>;
}
