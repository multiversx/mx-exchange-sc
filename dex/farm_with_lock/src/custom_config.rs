elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm::module]
pub trait CustomConfigModule: config::ConfigModule + token_send::TokenSendModule {
    #[view(getPairContractManagedAddress)]
    #[storage_mapper("pair_contract_address")]
    fn pair_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getLockedAssetFactoryManagedAddress)]
    #[storage_mapper("locked_asset_factory_address")]
    fn locked_asset_factory_address(&self) -> SingleValueMapper<ManagedAddress>;
}
