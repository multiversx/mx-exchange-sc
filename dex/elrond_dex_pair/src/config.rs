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

    #[storage_mapper("send_fee_gas_limit")]
    fn send_fee_gas_limit(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("extern_swap_gas_limit")]
    fn extern_swap_gas_limit(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("transfer_exec_gas_limit")]
    fn transfer_exec_gas_limit(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[endpoint]
    fn set_transfer_exec_gas_limit(&self, gas_limit: u64) -> SCResult<()> {
        self.require_permissions()?;
        self.transfer_exec_gas_limit().set(&gas_limit);
        Ok(())
    }

    fn require_permissions(&self) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let owner = self.router_owner_address().get();
        let router = self.router_address().get();
        require!(caller == owner || caller == router, "Permission denied");
        Ok(())
    }
}
