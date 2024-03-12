multiversx_sc::imports!();

const GAS_FOR_END_TX: u64 = 10_000;

#[multiversx_sc::module]
pub trait CreatePairModule: energy_query::EnergyQueryModule {
    #[only_owner]
    #[endpoint(clearTokenInfo)]
    fn clear_token_info(&self, token_id: TokenIdentifier) {
        self.requested_price(&token_id).clear();
        self.pair_for_token(&token_id).clear();
    }

    #[payable("*")]
    #[endpoint(depositProjectToken)]
    fn deposit_project_token(&self, requested_mex_price: BigUint) {
        let (token_id, _) = self.call_value().single_fungible_esdt();
        self.requested_price(&token_id).update(|price| {
            require!(*price == 0, "Price already set");

            *price = requested_mex_price;
        });
    }

    #[payable("EGLD")]
    #[endpoint(createXmexTokenPair)]
    fn create_xmex_token_pair(&self, token_id: TokenIdentifier) {
        require!(
            !self.requested_price(&token_id).is_empty(),
            "Tokens not deposited"
        );
        require!(
            self.pair_for_token(&token_id).is_empty(),
            "Pair already created"
        );
    }

    fn create_pair(&self, token_id: TokenIdentifier, caller: ManagedAddress) -> ManagedAddress {
        let mex_token_id = self.get_base_token_id();
        let own_sc_address = self.blockchain().get_sc_address();
        let router_address = self.router_address().get();
        let mut admins = MultiValueEncoded::new();
        admins.push(caller);

        self.router_proxy(router_address)
            .create_pair_endpoint(
                token_id,
                mex_token_id,
                own_sc_address,
                OptionalValue::Some(MultiValue2::from((0u64, 0u64))),
                admins,
            )
            .execute_on_dest_context()
    }

    #[proxy]
    fn router_proxy(&self, sc_address: ManagedAddress) -> router::Proxy<Self::Api>;

    #[storage_mapper("routerAddr")]
    fn router_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getRequestedPrice)]
    #[storage_mapper("reqPrice")]
    fn requested_price(&self, token_id: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    #[view(getPairForToken)]
    #[storage_mapper("pairForToken")]
    fn pair_for_token(&self, token_id: &TokenIdentifier) -> SingleValueMapper<ManagedAddress>;
}
