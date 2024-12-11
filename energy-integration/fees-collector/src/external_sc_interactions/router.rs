multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq)]
pub struct PairTokens<M: ManagedTypeApi> {
    pub first_token_id: TokenIdentifier<M>,
    pub second_token_id: TokenIdentifier<M>,
}

#[multiversx_sc::module]
pub trait RouterInteractionsModule: crate::config::ConfigModule + utils::UtilsModule {
    #[only_owner]
    #[endpoint(setRouterAddress)]
    fn set_router_address(&self, router_address: ManagedAddress) {
        self.require_sc_address(&router_address);

        self.router_address().set(router_address);
    }

    #[only_owner]
    #[endpoint(setBaseTokenId)]
    fn set_base_token_id(&self, base_token_id: TokenIdentifier) {
        self.require_valid_token_id(&base_token_id);

        self.base_token_id().set(base_token_id);
    }

    // Mimics the "get_pair" logic from router. Way cheaper than doing an external call.
    fn get_pair(&self, other_token_id: TokenIdentifier) -> Option<ManagedAddress> {
        let base_token_id = self.base_token_id().get();
        if other_token_id == base_token_id {
            return None;
        }

        let router_address = self.router_address().get();
        let pair_map_mapper = self.pair_map(router_address);

        let opt_address = pair_map_mapper.get(&PairTokens {
            first_token_id: other_token_id.clone(),
            second_token_id: base_token_id.clone(),
        });
        if opt_address.is_some() {
            return opt_address;
        }

        pair_map_mapper.get(&PairTokens {
            first_token_id: base_token_id,
            second_token_id: other_token_id,
        })
    }

    #[storage_mapper("routerAddress")]
    fn router_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("baseTokenId")]
    fn base_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    // router storage

    #[storage_mapper_from_address("pair_map")]
    fn pair_map(
        &self,
        router_address: ManagedAddress,
    ) -> MapMapper<PairTokens<Self::Api>, ManagedAddress, ManagedAddress>;
}
