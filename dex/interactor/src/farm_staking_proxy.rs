#![allow(unused)]

use multiversx_sc_scenario::{
    api::StaticApi,
    imports::{
        Address, BigUint, EsdtTokenPayment, ManagedAddress, ManagedVec, ReturnsResult, RustBigUint,
    },
};
use multiversx_sc_snippets::InteractorPrepareAsync;
use proxies::farm_staking_proxy_sc_proxy;

use crate::{
    structs::{
        extract_caller, InteractorClaimDualYieldResult, InteractorStakeProxyResult,
        InteractorToken, InteractorUnstakeResult,
    },
    DexInteract,
};

// views
pub(crate) async fn is_sc_address_whitelisted(dex_interact: &mut DexInteract, address: Address) -> bool {
    dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .is_sc_address_whitelisted(ManagedAddress::from(address))
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await
}

pub(crate) async fn get_dual_yield_token_id(dex_interact: &mut DexInteract) -> String {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .dual_yield_token()
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    result_token.to_string()
}

pub(crate) async fn get_lp_farm_address(dex_interact: &mut DexInteract) -> Address {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .lp_farm_address()
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    result_token.to_address()
}

pub(crate) async fn get_staking_farm_address(dex_interact: &mut DexInteract) -> Address {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .staking_farm_address()
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    result_token.to_address()
}

pub(crate) async fn get_pair_address(dex_interact: &mut DexInteract) -> Address {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .pair_address()
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    result_token.to_address()
}

pub(crate) async fn get_staking_token_id(dex_interact: &mut DexInteract) -> String {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .staking_token_id()
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    result_token.to_string()
}

pub(crate) async fn get_farm_token_id(dex_interact: &mut DexInteract) -> String {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .staking_farm_token_id()
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    result_token.to_string()
}

pub(crate) async fn get_lp_token_id(dex_interact: &mut DexInteract) -> String {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .lp_token_id()
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    result_token.to_string()
}

pub(crate) async fn get_lp_farm_token_id(dex_interact: &mut DexInteract) -> String {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .lp_farm_token_id()
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    result_token.to_string()
}

pub(crate) async fn get_energy_factory_address(dex_interact: &mut DexInteract) -> Address {
    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .energy_factory_address()
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    result_token.to_address()
}

// endpoints
pub(crate) async fn stake_farm_tokens(
    dex_interact: &mut DexInteract,
    payment: Vec<InteractorToken>,
    opt_original_caller: Option<Address>,
) -> InteractorStakeProxyResult {
    let caller_arg = extract_caller(dex_interact, opt_original_caller);

    let payments = payment
        .iter()
        .map(EsdtTokenPayment::from)
        .collect::<ManagedVec<StaticApi, EsdtTokenPayment<StaticApi>>>();

    let result_token = dex_interact
        .interactor
        .tx()
        .from(&dex_interact.wallet_address)
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .gas(100_000_000u64)
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .stake_farm_tokens(caller_arg)
        .payment(payments)
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    InteractorStakeProxyResult::from(result_token)
}

pub(crate) async fn claim_dual_yield(
    dex_interact: &mut DexInteract,
    payment: InteractorToken,
    opt_original_caller: Option<Address>,
) -> InteractorClaimDualYieldResult {
    let caller_arg = extract_caller(dex_interact, opt_original_caller);

    let result_token = dex_interact
        .interactor
        .tx()
        .from(&dex_interact.wallet_address)
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .gas(100_000_000u64)
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .claim_dual_yield_endpoint(caller_arg)
        .payment(EsdtTokenPayment::from(payment))
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    InteractorClaimDualYieldResult::from(result_token)
}

pub(crate) async fn unstake_farm_tokens(
    dex_interact: &mut DexInteract,
    payment: InteractorToken,
    pair_first_token_min_amount: RustBigUint,
    pair_second_token_min_amount: RustBigUint,
    opt_original_caller: Option<Address>,
) -> InteractorUnstakeResult {
    let caller_arg = extract_caller(dex_interact, opt_original_caller);

    let result_token = dex_interact
        .interactor
        .tx()
        .from(&dex_interact.wallet_address)
        .to(dex_interact.state.current_farm_staking_proxy_address())
        .gas(100_000_000u64)
        .typed(farm_staking_proxy_sc_proxy::FarmStakingProxyProxy)
        .unstake_farm_tokens(
            BigUint::from(pair_first_token_min_amount),
            BigUint::from(pair_second_token_min_amount),
            caller_arg,
        )
        .payment(EsdtTokenPayment::from(payment))
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    InteractorUnstakeResult::from(result_token)
}
