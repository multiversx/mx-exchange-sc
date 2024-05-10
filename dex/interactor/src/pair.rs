use multiversx_sc_scenario::imports::{BigUint, ReturnsResult, TokenIdentifier};
use multiversx_sc_snippets::InteractorPrepareAsync;
use proxies::pair_proxy;

use crate::{
    dex_interact_cli::{AddArgs, SwapArgs},
    structs::{InteractorAddLiquidityResultType, InteractorToken},
    DexInteract,
};

pub(crate) async fn swap_tokens_fixed_input(
    dex_interact: &mut DexInteract,
    args: &SwapArgs,
) -> InteractorToken {
    let payment = args.as_payment(dex_interact);
    let first_token_id = dex_interact.state.first_token_id();
    let second_token_id = dex_interact.state.second_token_id();

    println!(
        "Attempting to swap {} {first_token_id} for a min amount {} of {second_token_id}...",
        args.amount, args.min_amount
    );

    let result_token = dex_interact
        .interactor
        .tx()
        .from(&dex_interact.wallet_address)
        .to(dex_interact.state.current_pair_address())
        .gas(100_000_000u64)
        .typed(pair_proxy::PairProxy)
        .swap_tokens_fixed_input(
            TokenIdentifier::from(second_token_id.as_bytes()),
            BigUint::from(args.min_amount),
        )
        .payment(payment)
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    InteractorToken::from(result_token)
}

pub(crate) async fn add_liquidity(
    dex_interact: &mut DexInteract,
    args: &AddArgs,
) -> InteractorAddLiquidityResultType {
    println!("Attempting to add liquidity to pair...");
    let payments = args.as_payment_vec(dex_interact);

    let result_token = dex_interact
        .interactor
        .tx()
        .from(&dex_interact.wallet_address)
        .to(dex_interact.state.current_pair_address())
        .gas(100_000_000u64)
        .typed(pair_proxy::PairProxy)
        .add_liquidity(
            BigUint::from(args.first_token_amount_min),
            BigUint::from(args.second_token_amount_min),
        )
        .payment(payments)
        .returns(ReturnsResult)
        .prepare_async()
        .run()
        .await;

    InteractorAddLiquidityResultType::from(result_token)
}

// 10000000000000000000 ; 10 UTK
// 1000000000000; 0,000001 WEGLD
// cargo run swap -a 10000000000000000000 -m 1000000000000
