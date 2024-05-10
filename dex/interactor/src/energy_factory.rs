#![allow(unused)]

use multiversx_sc_scenario::imports::{
    Address, BigUint, ManagedAddress, ReturnsResult, RustBigUint,
};
use multiversx_sc_snippets::InteractorPrepareAsync;
use proxies::energy_factory_proxy;

use crate::{
    structs::{to_rust_biguint, InteractorEnergy},
    DexInteract,
};

pub(crate) async fn get_energy_entry_for_user(
    dex_interact: &mut DexInteract,
    user: Address,
) -> InteractorEnergy {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_energy_factory_address())
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .get_updated_energy_entry_for_user(ManagedAddress::from(user))
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    InteractorEnergy::from(result_token)
}

pub(crate) async fn get_energy_amount_for_user(
    dex_interact: &mut DexInteract,
    user: Address,
) -> RustBigUint {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_energy_factory_address())
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .get_energy_amount_for_user(ManagedAddress::from(user))
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    to_rust_biguint(result_token)
}

pub(crate) async fn get_penalty_amount(
    dex_interact: &mut DexInteract,
    token_amount: RustBigUint,
    prev_lock_epochs: u64,
    new_lock_epochs: u64,
) -> RustBigUint {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_energy_factory_address())
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .calculate_penalty_amount(
            BigUint::from(token_amount),
            prev_lock_epochs,
            new_lock_epochs,
        )
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    to_rust_biguint(result_token)
}

pub(crate) async fn get_token_unstake_address(dex_interact: &mut DexInteract) -> Address {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_energy_factory_address())
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .token_unstake_sc_address()
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    result_token.to_address()
}

pub(crate) async fn is_sc_address_whitelisted(
    dex_interact: &mut DexInteract,
    address: Address,
) -> bool {
    dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_energy_factory_address())
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .is_sc_address_whitelisted(ManagedAddress::from(address))
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await
}
