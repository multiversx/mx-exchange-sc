elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm_derive::module]
pub trait UtilModule {
    #[inline]
    fn is_active(&self) -> bool {
        self.state().get()
    }

    #[endpoint(setPairCreationEnabled)]
    fn set_pair_creation_enabled(&self, enabled: bool) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.pair_creation_enabled().set(&enabled);
        Ok(())
    }

    #[view(getOwner)]
    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<Self::Storage, bool>;

    #[view(getPairCreationEnabled)]
    #[storage_mapper("pair_creation_enabled")]
    fn pair_creation_enabled(&self) -> SingleValueMapper<Self::Storage, bool>;

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<Self::Storage, BoxedBytes>;
}
