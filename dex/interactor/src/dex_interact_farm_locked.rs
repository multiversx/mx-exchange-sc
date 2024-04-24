use multiversx_sc_scenario::{
    api::StaticApi,
    imports::{
        Address, EsdtTokenPayment, ManagedAddress, ManagedVec, OptionalValue, ReturnsResult,
    },
};
use multiversx_sc_snippets::InteractorPrepareAsync;

use crate::{farm_with_locked_rewards_proxy, structs::InteractorToken, DexInteract};

pub struct FarmLocked;

pub trait FarmLockedTrait {
    async fn enter_farm(
        dex_interact: &mut DexInteract,
        lp_token: InteractorToken,
    ) -> (InteractorToken, InteractorToken);
    async fn claim_rewards(
        dex_interact: &mut DexInteract,
        payment: Vec<InteractorToken>,
        opt_original_caller: Option<Address>,
    ) -> (InteractorToken, InteractorToken);
    async fn exit_farm(
        dex_interact: &mut DexInteract,
        payment: InteractorToken,
        opt_original_caller: Option<Address>,
    ) -> (InteractorToken, InteractorToken);
    async fn merge_farm_tokens(
        dex_interact: &mut DexInteract,
        payment: Vec<InteractorToken>,
        opt_original_caller: Option<Address>,
    ) -> (InteractorToken, InteractorToken);
}

impl FarmLockedTrait for FarmLocked {
    async fn enter_farm(
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
            .enter_farm_endpoint(ManagedAddress::from(
                dex_interact.wallet_address.as_address(),
            ))
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

    async fn claim_rewards(
        dex_interact: &mut DexInteract,
        payment: Vec<InteractorToken>,
        opt_original_caller: Option<Address>,
    ) -> (InteractorToken, InteractorToken) {
        println!("Attempting to claim rewards from farm with locked rewards...");

        let payments = payment
            .iter()
            .map(EsdtTokenPayment::from)
            .collect::<ManagedVec<StaticApi, EsdtTokenPayment<StaticApi>>>();

        let caller =
            opt_original_caller.unwrap_or_else(|| dex_interact.wallet_address.to_address());
        let caller_arg: OptionalValue<ManagedAddress<StaticApi>> =
            OptionalValue::Some(ManagedAddress::from(caller));

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
            .payment(&payments)
            .returns(ReturnsResult)
            .prepare_async()
            .run()
            .await;

        (
            InteractorToken::from(result_token.0 .0),
            InteractorToken::from(result_token.0 .1),
        )
    }

    async fn exit_farm(
        dex_interact: &mut DexInteract,
        payment: InteractorToken,
        opt_original_caller: Option<Address>,
    ) -> (InteractorToken, InteractorToken) {
        println!("Attempting to exit farm with locked rewards...");

        let caller =
            opt_original_caller.unwrap_or_else(|| dex_interact.wallet_address.to_address());
        let caller_arg: OptionalValue<ManagedAddress<StaticApi>> =
            OptionalValue::Some(ManagedAddress::from(caller));

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

    async fn merge_farm_tokens(
        dex_interact: &mut DexInteract,
        payment: Vec<InteractorToken>,
        opt_original_caller: Option<Address>,
    ) -> (InteractorToken, InteractorToken) {
        println!("Attempting to merge tokens in farm with locked rewards...");

        let payments = payment
            .iter()
            .map(EsdtTokenPayment::from)
            .collect::<ManagedVec<StaticApi, EsdtTokenPayment<StaticApi>>>();

        let caller =
            opt_original_caller.unwrap_or_else(|| dex_interact.wallet_address.to_address());
        let caller_arg: OptionalValue<ManagedAddress<StaticApi>> =
            OptionalValue::Some(ManagedAddress::from(caller));

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
}
