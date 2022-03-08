use crate::redeem_token::{ACCEPTED_TOKEN_REDEEM_NONCE, LAUNCHED_TOKEN_REDEEM_NONCE};

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
pub trait CreatePoolModule: crate::common_storage::CommonStorageModule {
    #[only_owner]
    #[endpoint(setPairAddress)]
    fn set_pair_address(&self, dex_sc_address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&dex_sc_address),
            "Invalid DEX SC address"
        );
        self.dex_sc_address().set(&dex_sc_address);
    }

    #[endpoint(createDexLiquidityPool)]
    fn create_dex_liquidity_pool(&self) {
        require!(!self.dex_sc_address().is_empty(), "Pair address not set");
        require!(self.lp_token_id().is_empty(), "Pool already created");
        self.require_deposit_period_ended();

        let launched_token_id = self.launched_token_id().get();
        let accepted_token_id = self.accepted_token_id().get();
        let extra_rewards_token_id = self.extra_rewards_token_id().get();

        let launched_token_balance = self.blockchain().get_sc_balance(&launched_token_id, 0);
        let accepted_token_balance = self.blockchain().get_sc_balance(&accepted_token_id, 0);
        let extra_rewards_balance = self.blockchain().get_sc_balance(&extra_rewards_token_id, 0);
        let launched_token_accumulated_penalty =
            self.accumulated_penalty(LAUNCHED_TOKEN_REDEEM_NONCE).get();
        let accepted_token_accumulated_penalty =
            self.accumulated_penalty(ACCEPTED_TOKEN_REDEEM_NONCE).get();

        require!(
            launched_token_balance > 0,
            "No Launched tokens were deposited"
        );
        require!(accepted_token_balance > 0, "No users deposited tokens");

        let launched_token_final_amount =
            &launched_token_balance - &launched_token_accumulated_penalty;
        let accepted_token_final_amount =
            &accepted_token_balance - &accepted_token_accumulated_penalty;
        let extra_rewards_final_amount = extra_rewards_balance;
        self.launched_token_final_amount()
            .set(&launched_token_final_amount);
        self.accepted_token_final_amount()
            .set(&accepted_token_final_amount);
        self.extra_rewards_final_amount()
            .set(&extra_rewards_final_amount);

        let mut payments = ManagedVec::<Self::Api, EsdtTokenPayment<Self::Api>>::new();
        payments.push(EsdtTokenPayment {
            token_type: EsdtTokenType::Fungible,
            token_identifier: launched_token_id,
            token_nonce: 0,
            amount: launched_token_balance.clone(),
        });
        payments.push(EsdtTokenPayment {
            token_type: EsdtTokenType::Fungible,
            token_identifier: accepted_token_id,
            token_nonce: 0,
            amount: accepted_token_balance.clone(),
        });

        let dex_sc_address = self.dex_sc_address().get();
        let contract_call = self
            .dex_proxy(dex_sc_address)
            .add_initial_liquidity(payments, OptionalValue::None);

        let (lp_token, _, _) = contract_call.execute_on_dest_context().into_tuple();
        let extra_lp_tokens = self.calculate_extra_lp_tokens(
            &launched_token_balance,
            &accepted_token_balance,
            &launched_token_accumulated_penalty,
            &accepted_token_accumulated_penalty,
            &lp_token.amount,
        );

        self.lp_token_id().set(&lp_token.token_identifier);
        self.extra_lp_tokens().set(&extra_lp_tokens);
        self.total_lp_tokens_received()
            .set(&(lp_token.amount - extra_lp_tokens));

        let current_epoch = self.blockchain().get_block_epoch();
        self.pool_creation_epoch().set(&current_epoch);
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

    fn calculate_extra_lp_tokens(
        &self,
        launched_token_final_amount: &BigUint,
        accepted_token_final_amount: &BigUint,
        launched_token_accumulated_penalty: &BigUint,
        accepted_token_accumulated_penalty: &BigUint,
        total_lp_tokens: &BigUint,
    ) -> BigUint {
        let unusable_lp_tokens_for_launched_tokens = &(launched_token_accumulated_penalty
            * total_lp_tokens)
            / launched_token_final_amount
            / 2u32;
        let unusable_lp_tokens_for_accepted_tokens = &(accepted_token_accumulated_penalty
            * total_lp_tokens)
            / accepted_token_final_amount
            / 2u32;

        unusable_lp_tokens_for_launched_tokens + unusable_lp_tokens_for_accepted_tokens
    }

    #[proxy]
    fn dex_proxy(&self, sc_address: ManagedAddress) -> liquidity_pool_proxy::Proxy<Self::Api>;

    #[view(getDexScAddress)]
    #[storage_mapper("dexScAddress")]
    fn dex_sc_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("poolCreationEpoch")]
    fn pool_creation_epoch(&self) -> SingleValueMapper<u64>;
}
