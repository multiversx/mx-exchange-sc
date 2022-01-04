elrond_wasm::imports!();

const ACCEPT_FUNDS_FUNC_NAME: &[u8] = b"accept_funds_func";

mod liquidity_pool_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait LiquidityPool {
        #[payable("*")]
        #[endpoint(addInitialLiquidity)]
        fn add_initial_liquidity(
            &self,
            #[payment_multi] payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
            #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
        ) -> MultiResult3<
            EsdtTokenPayment<Self::Api>,
            EsdtTokenPayment<Self::Api>,
            EsdtTokenPayment<Self::Api>,
        >;
    }
}

#[elrond_wasm::module]
pub trait CreatePoolModule: crate::common_storage::CommonStorageModule {
    #[endpoint(createDexLiquidityPool)]
    fn create_dex_liquidity_pool(&self) -> SCResult<()> {
        require!(self.lp_token_id().is_empty(), "Pool already created");
        self.require_deposit_period_ended()?;

        let launched_token_id = self.launched_token_id().get();
        let accepted_token_id = self.accepted_token_id().get();

        let launched_token_balance = self.blockchain().get_sc_balance(&launched_token_id, 0);
        let accepted_token_balance = self.blockchain().get_sc_balance(&accepted_token_id, 0);

        self.launched_token_final_amount()
            .set(&launched_token_balance);
        self.accepted_token_final_amount()
            .set(&accepted_token_balance);

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
            .add_initial_liquidity(payments, OptionalArg::Some(ACCEPT_FUNDS_FUNC_NAME.into()));

        let (_, _, lp_token) = contract_call.execute_on_dest_context().into_tuple();
        self.lp_token_id().set(&lp_token.token_identifier);
        self.total_lp_tokens_received().set(&lp_token.amount);

        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn accept_funds_func(&self) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let dex_sc_address = self.dex_sc_address().get();
        require!(
            caller == dex_sc_address,
            "Only the DEX SC may call this function"
        );

        Ok(())
    }

    // private

    fn require_deposit_period_ended(&self) -> SCResult<()> {
        let current_epoch = self.blockchain().get_block_epoch();
        let end_epoch = self.end_epoch().get();
        require!(current_epoch >= end_epoch, "Deposit period has not ended");

        Ok(())
    }

    #[proxy]
    fn dex_proxy(&self, sc_address: ManagedAddress) -> liquidity_pool_proxy::Proxy<Self::Api>;

    #[view(getDexScAddress)]
    #[storage_mapper("dexScAddress")]
    fn dex_sc_address(&self) -> SingleValueMapper<ManagedAddress>;
}
