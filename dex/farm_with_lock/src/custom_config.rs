elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[macro_export]
macro_rules! assert {
    ($self:expr, $cond:expr, $msg:expr $(,)?) => {
        if !$cond {
            assert!($self, $msg)
        }
    };
    ($self:expr, $msg:expr $(,)?) => {
        $self.raw_vm_api().signal_error($msg)
    };
}

#[elrond_wasm::module]
pub trait CustomConfigModule: config::ConfigModule + token_send::TokenSendModule {
    #[view(getLockedAssetFactoryManagedAddress)]
    #[storage_mapper("locked_asset_factory_address")]
    fn locked_asset_factory_address(&self) -> SingleValueMapper<ManagedAddress>;
}
