#![allow(unused)]

use common_structs::FarmTokenAttributes;

use multiversx_sc_snippets::imports::*;

use crate::{
    farm_with_locked_rewards_proxy,
    structs::{extract_caller, to_rust_biguint, InteractorFarmTokenAttributes, InteractorToken},
    DexInteract,
};

pub(crate) async fn enter_farm(
    dex_interact: &mut DexInteract,
    lp_token: InteractorToken,
) -> (InteractorToken, InteractorToken) {
    println!("Attempting to enter farm with locked rewards...");

    let result_token = dex_interact
        .interactor
        .tx()
        .from(&dex_interact.wallet_address)
        .to(dex_interact
            .state
            .current_farm_with_locked_rewards_address())
        .gas(100_000_000u64)
        .typed(farm_with_locked_rewards_proxy::FarmProxy)
        .enter_farm_endpoint(OptionalValue::Some(ManagedAddress::from(
            dex_interact.wallet_address.as_address(),
        )))
        .payment::<EsdtTokenPayment<StaticApi>>(lp_token.into())
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;
    (
        InteractorToken::from(result_token.0 .0),
        InteractorToken::from(result_token.0 .1),
    )
}

pub(crate) async fn claim_rewards(
    dex_interact: &mut DexInteract,
    payment: Vec<InteractorToken>,
    opt_original_caller: Option<Address>,
) -> (InteractorToken, InteractorToken) {
    println!("Attempting to claim rewards from farm with locked rewards...");

    let payments = payment
        .iter()
        .map(EsdtTokenPayment::from)
        .collect::<ManagedVec<StaticApi, EsdtTokenPayment<StaticApi>>>();

    let caller_arg = extract_caller(dex_interact, opt_original_caller);

    let result_token = dex_interact
        .interactor
        .tx()
        .from(&dex_interact.wallet_address)
        .to(dex_interact
            .state
            .current_farm_with_locked_rewards_address())
        .gas(100_000_000u64)
        .typed(farm_with_locked_rewards_proxy::FarmProxy)
        .claim_rewards_endpoint(caller_arg)
        .payment(payments)
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    (
        InteractorToken::from(result_token.0 .0),
        InteractorToken::from(result_token.0 .1),
    )
}

pub(crate) async fn exit_farm(
    dex_interact: &mut DexInteract,
    payment: InteractorToken,
    opt_original_caller: Option<Address>,
) -> (InteractorToken, InteractorToken) {
    println!("Attempting to exit farm with locked rewards...");

    let caller_arg = extract_caller(dex_interact, opt_original_caller);

    let result_token = dex_interact
        .interactor
        .tx()
        .from(&dex_interact.wallet_address)
        .to(dex_interact
            .state
            .current_farm_with_locked_rewards_address())
        .gas(100_000_000u64)
        .typed(farm_with_locked_rewards_proxy::FarmProxy)
        .exit_farm_endpoint(caller_arg)
        .payment::<EsdtTokenPayment<StaticApi>>(payment.into())
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    (
        InteractorToken::from(result_token.0 .0),
        InteractorToken::from(result_token.0 .1),
    )
}

pub(crate) async fn merge_farm_tokens(
    dex_interact: &mut DexInteract,
    payment: Vec<InteractorToken>,
    opt_original_caller: Option<Address>,
) -> (InteractorToken, InteractorToken) {
    println!("Attempting to merge tokens in farm with locked rewards...");

    let payments = payment
        .iter()
        .map(EsdtTokenPayment::from)
        .collect::<ManagedVec<StaticApi, EsdtTokenPayment<StaticApi>>>();

    let caller_arg = extract_caller(dex_interact, opt_original_caller);

    let result_token = dex_interact
        .interactor
        .tx()
        .from(&dex_interact.wallet_address)
        .to(dex_interact
            .state
            .current_farm_with_locked_rewards_address())
        .gas(100_000_000u64)
        .typed(farm_with_locked_rewards_proxy::FarmProxy)
        .merge_farm_tokens_endpoint(caller_arg)
        .payment(payments)
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    (
        InteractorToken::from(result_token.0 .0),
        InteractorToken::from(result_token.0 .1),
    )
}

pub(crate) async fn claim_boosted_rewards(
    dex_interact: &mut DexInteract,
    opt_original_caller: Option<Address>,
) -> InteractorToken {
    let caller_arg = extract_caller(dex_interact, opt_original_caller);

    let result_token = dex_interact
        .interactor
        .tx()
        .from(&dex_interact.wallet_address)
        .to(dex_interact
            .state
            .current_farm_with_locked_rewards_address())
        .gas(100_000_000u64)
        .typed(farm_with_locked_rewards_proxy::FarmProxy)
        .claim_boosted_rewards(caller_arg)
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    InteractorToken::from(result_token)
}

pub(crate) async fn calculate_rewards_for_given_position(
    dex_interact: &mut DexInteract,
    user: Address,
    farm_token_amount: u128,
    attributes: InteractorFarmTokenAttributes,
) -> RustBigUint {
    let attributes_arg: FarmTokenAttributes<StaticApi> = FarmTokenAttributes {
        reward_per_share: BigUint::from(attributes.reward_per_share),
        entering_epoch: attributes.entering_epoch,
        compounded_reward: BigUint::from(attributes.compounded_reward),
        current_farm_amount: BigUint::from(attributes.current_farm_amount),
        original_owner: ManagedAddress::from(attributes.original_owner),
    };

    let result_token = dex_interact
        .interactor
        .query()
        .to(dex_interact
            .state
            .current_farm_with_locked_rewards_address())
        .typed(farm_with_locked_rewards_proxy::FarmProxy)
        .calculate_rewards_for_given_position(
            ManagedAddress::from(user),
            BigUint::from(farm_token_amount),
            attributes_arg,
        )
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    to_rust_biguint(result_token)
}
