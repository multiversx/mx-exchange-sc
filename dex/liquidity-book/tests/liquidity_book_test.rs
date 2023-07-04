mod liquidity_book_setup;
use liquidity_book::PRICE_DECIMALS;
use liquidity_book_setup::*;

#[test]
fn test_liquidity_book_setup() {
    let deploy_price = 5000 * PRICE_DECIMALS as u128;
    let _ = LiquidityBookSetup::new(deploy_price, liquidity_book::contract_obj);
}

#[test]
fn test_add_remove_liquidity() {
    let deploy_price = 5000 * PRICE_DECIMALS as u128;
    let mut pair_setup = LiquidityBookSetup::new(deploy_price, liquidity_book::contract_obj);
    let user = pair_setup.setup_user(USER_TOTAL_WEGLD_TOKENS, USER_TOTAL_MEX_TOKENS);
    let first_token_amount = 1 * PRICE_DECIMALS as u128;
    let first_token_dust = 1023381652573612u128;
    let second_token_amount = 5_000 * PRICE_DECIMALS as u128;
    let second_token_dust = 2u128;
    let expected_lp_amount = 1517882343751510417954u128;
    pair_setup.add_liquidity(
        &user,
        4545 * PRICE_DECIMALS as u128, //tick - 84222, // price - 4545
        5500 * PRICE_DECIMALS as u128, //tick - 86129, // price - 5500
        first_token_amount,
        second_token_amount,
        1,
        expected_lp_amount,
        first_token_dust,
        second_token_dust,
    );
    pair_setup.remove_liquidity(
        &user,
        1,
        expected_lp_amount,
        first_token_amount - first_token_dust,
        second_token_amount - second_token_dust,
    );
}

#[test]
fn test_swap_inside_same_tick() {
    let deploy_price = 5000 * PRICE_DECIMALS as u128;
    let mut pair_setup = LiquidityBookSetup::new(deploy_price, liquidity_book::contract_obj);
    let user = pair_setup.setup_user(USER_TOTAL_WEGLD_TOKENS, USER_TOTAL_MEX_TOKENS);
    let first_token_amount = 1 * PRICE_DECIMALS as u128;
    let first_token_dust = 1023381652573612u128;
    let second_token_amount = 5_000 * PRICE_DECIMALS as u128;
    let second_token_dust = 2u128;
    let expected_lp_amount = 1517882343751510417954u128;
    pair_setup.add_liquidity(
        &user,
        4545 * PRICE_DECIMALS as u128, //tick - 84222, // price - 4545
        5500 * PRICE_DECIMALS as u128, //tick - 86129, // price - 5500
        first_token_amount,
        second_token_amount,
        1,
        expected_lp_amount,
        first_token_dust,
        second_token_dust,
    );

    let swap_input_amount = 42 * PRICE_DECIMALS as u128;
    let swap_expected_output_amount = 8396714242162445u128;
    let swap_fee_expected_amount = 25190142726487u128;
    pair_setup.swap_tokens(
        &user,
        MEX_TOKEN_ID,
        swap_input_amount,
        WEGLD_TOKEN_ID,
        swap_expected_output_amount - swap_fee_expected_amount,
    );
}

#[test]
fn test_swap_inside_same_tick_with_deeper_liquidity() {
    let deploy_price = 5000 * PRICE_DECIMALS as u128;
    let mut pair_setup = LiquidityBookSetup::new(deploy_price, liquidity_book::contract_obj);
    let user = pair_setup.setup_user(USER_TOTAL_WEGLD_TOKENS, USER_TOTAL_MEX_TOKENS);
    let first_token_amount = 1 * PRICE_DECIMALS as u128;
    let first_token_dust = 1023381652573612u128;
    let second_token_amount = 5_000 * PRICE_DECIMALS as u128;
    let second_token_dust = 2u128;
    let expected_lp_amount = 1517882343751510417954u128;
    pair_setup.add_liquidity(
        &user,
        4545 * PRICE_DECIMALS as u128, //tick - 84222, // price - 4545
        5500 * PRICE_DECIMALS as u128, //tick - 86129, // price - 5500
        first_token_amount,
        second_token_amount,
        1,
        expected_lp_amount,
        first_token_dust,
        second_token_dust,
    );
    pair_setup.add_liquidity(
        &user,
        4545 * PRICE_DECIMALS as u128, //tick - 84222, // price - 4545
        5500 * PRICE_DECIMALS as u128, //tick - 86129, // price - 5500
        first_token_amount.clone(),
        second_token_amount.clone(),
        2,
        expected_lp_amount.clone(),
        first_token_dust.clone(),
        second_token_dust,
    );

    let swap_input_amount = 42 * PRICE_DECIMALS as u128;
    let swap_expected_output_amount = 8398356799702754u128;
    let swap_fee_expected_amount = 25195070399108u128;
    pair_setup.swap_tokens(
        &user,
        MEX_TOKEN_ID,
        swap_input_amount,
        WEGLD_TOKEN_ID,
        swap_expected_output_amount - swap_fee_expected_amount, //8373161729303646
    );
}

#[test]
fn test_swap_consecutive_price_ranges() {
    let deploy_price = 5000 * PRICE_DECIMALS as u128;
    let mut pair_setup = LiquidityBookSetup::new(deploy_price, liquidity_book::contract_obj);
    let user = pair_setup.setup_user(USER_TOTAL_WEGLD_TOKENS, USER_TOTAL_MEX_TOKENS);
    let first_token_amount = 1 * PRICE_DECIMALS as u128;
    let first_token_dust = 1023381652573612u128;
    let second_token_amount = 5_000 * PRICE_DECIMALS as u128;
    let second_token_dust = 2u128;
    let expected_lp_amount = 1517882343751510417954u128;
    pair_setup.add_liquidity(
        &user,
        4545 * PRICE_DECIMALS as u128, //tick - 84222, // price - 4545
        5500 * PRICE_DECIMALS as u128, //tick - 86129, // price - 5500
        first_token_amount,
        second_token_amount,
        1,
        expected_lp_amount,
        first_token_dust,
        second_token_dust,
    );

    let expected_lp_amount_2 = 669781156610111695400u128;
    let add2_first_token_dust = 1u128;
    let add2_second_token_dust = 2688379771626960516478u128;
    pair_setup.add_liquidity(
        &user,
        5500 * PRICE_DECIMALS as u128, // tick - 86129
        6250 * PRICE_DECIMALS as u128, // tick - 87407
        first_token_amount,
        second_token_amount,
        2,
        expected_lp_amount_2,
        add2_first_token_dust,
        add2_second_token_dust,
    );

    let swap_input_amount = 10_000 * PRICE_DECIMALS as u128;
    let swap_expected_output_amount = 1844378376909598763;
    pair_setup.swap_tokens(
        &user,
        MEX_TOKEN_ID,
        swap_input_amount,
        WEGLD_TOKEN_ID,
        swap_expected_output_amount,
    );
}

#[test]
fn test_swap_consecutive_price_ranges_reverse_direction() {
    let mut pair_setup =
        LiquidityBookSetup::new(5000 * PRICE_DECIMALS as u128, liquidity_book::contract_obj);
    let user = pair_setup.setup_user(USER_TOTAL_WEGLD_TOKENS, USER_TOTAL_MEX_TOKENS);
    let first_token_amount = 1 * PRICE_DECIMALS as u128;
    let first_token_dust = 1023381652573612u128;
    let second_token_amount = 5_000 * PRICE_DECIMALS as u128;
    let second_token_dust = 2u128;
    let expected_lp_amount = 1517882343751510417954u128;
    pair_setup.add_liquidity(
        &user,
        4545 * PRICE_DECIMALS as u128, //tick - 84222, // price - 4545
        5500 * PRICE_DECIMALS as u128, //tick - 86129, // price - 5500
        first_token_amount,
        second_token_amount,
        1,
        expected_lp_amount,
        first_token_dust,
        second_token_dust,
    );

    let expected_lp_amount_2 = 669781156610111695400u128;
    let add2_first_token_dust = 537179194068028601u128;
    let add2_second_token_dust = 2u128;

    pair_setup.add_liquidity(
        &user,
        4000 * PRICE_DECIMALS as u128,
        4545 * PRICE_DECIMALS as u128,
        first_token_amount,
        second_token_amount,
        2,
        expected_lp_amount_2,
        add2_first_token_dust,
        add2_second_token_dust,
    );

    let swap_input_amount = 2 * PRICE_DECIMALS as u128;
    let swap_expected_output_amount = 9120263498059170314152u128;
    pair_setup.swap_tokens(
        &user,
        WEGLD_TOKEN_ID,
        swap_input_amount,
        MEX_TOKEN_ID,
        swap_expected_output_amount,
    );
}
