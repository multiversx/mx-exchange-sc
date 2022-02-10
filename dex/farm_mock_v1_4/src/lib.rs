#![no_std]

use common_structs::FarmTokenAttributes;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm::contract]
pub trait FarmMockV14 {
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint(migrateFromV1_2Farm)]
    fn migrate_from_v1_2_farm(
        &self,
        _attrs: FarmTokenAttributes<Self::Api>,
        _orig_caller: ManagedAddress,
    ) -> EsdtTokenPayment<Self::Api> {
        EsdtTokenPayment::new(TokenIdentifier::egld(), 0, BigUint::zero())
    }

    #[endpoint(setRpsAndStartRewards)]
    fn set_rps_and_start_rewards(&self, _rps: BigUint) {}
}
