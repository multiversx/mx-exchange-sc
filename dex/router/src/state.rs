elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm::module]
pub trait StateModule {
    #[inline]
    fn is_active(&self) -> bool {
        self.state().get()
    }

    #[only_owner]
    #[endpoint(setPairCreationEnabled)]
    fn set_pair_creation_enabled(&self, enabled: bool) -> SCResult<()> {
        self.pair_creation_enabled().set(&enabled);
        Ok(())
    }

    #[view(getOwner)]
    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<bool>;

    #[view(getPairCreationEnabled)]
    #[storage_mapper("pair_creation_enabled")]
    fn pair_creation_enabled(&self) -> SingleValueMapper<bool>;

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<ManagedBuffer>;
}
