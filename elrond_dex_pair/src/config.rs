elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub enum State {
    Inactive,
    Active,
    ActiveNoSwaps,
}

#[elrond_wasm_derive::module]
pub trait ConfigModule {
    #[view(getRouterAddress)]
    #[storage_mapper("router_address")]
    fn router_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getRouterOwnerAddress)]
    #[storage_mapper("router_owner_address")]
    fn router_owner_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<Self::Storage, State>;

    fn require_permissions(&self) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let owner = self.router_owner_address().get();
        let router = self.router_address().get();
        require!(caller == owner || caller == router, "Permission denied");
        Ok(())
    }
}
