#![allow(unused)]

use energy_interactor_proxies::{energy_factory_proxy, multisig_proxy};
use multiversx_sc_snippets::imports::*;

use crate::{
    structs::{extract_caller, to_rust_biguint, InteractorEnergy},
    DexInteract,
};

pub(crate) async fn propose_async_call(
    dex_interact: &mut DexInteract,
    to: Address,
    egld_amount: RustBigUint,
    opt_gas_limit: Option<u64>,
    function_call: &str,
) -> usize {
    let result_token = dex_interact
        .interactor
        .tx()
        .from(&dex_interact.wallet_address)
        .to(dex_interact.state.current_multisig_address())
        .gas(100_000_000u64)
        .typed(multisig_proxy::MultisigProxy)
        .propose_async_call(
            to,
            egld_amount,
            opt_gas_limit,
            FunctionCall::new(function_call),
        )
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    println!("Propose async call result {result_token}");

    result_token
}

pub(crate) async fn sign_and_perform(
    dex_interact: &mut DexInteract,
    action_id: usize,
) -> OptionalValue<ManagedAddress<StaticApi>> {
    let result_token = dex_interact
        .interactor
        .tx()
        .from(&dex_interact.second_wallet_address)
        .to(dex_interact.state.current_multisig_address())
        .gas(100_000_000u64)
        .typed(multisig_proxy::MultisigProxy)
        .sign_and_perform(action_id)
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

        
    result_token
}
