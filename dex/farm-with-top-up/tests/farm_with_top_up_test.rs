use farm_with_top_up_setup::{FarmWithTopUpSetup, REWARD_TOKEN_ID};
use multiversx_sc_scenario::rust_biguint;

pub mod farm_with_top_up_setup;

#[test]
fn setup_farm_with_top_up_test() {
    let _ = FarmWithTopUpSetup::new(
        farm_with_top_up::contract_obj,
        timestamp_oracle::contract_obj,
        permissions_hub::contract_obj,
    );
}

#[test]
fn admin_deposit_user_claim_test() {
    let mut setup = FarmWithTopUpSetup::new(
        farm_with_top_up::contract_obj,
        timestamp_oracle::contract_obj,
        permissions_hub::contract_obj,
    );

    let farm_token_amount = 100_000;
    let mut farm_token_nonce = 1;
    setup.user_enter_farm(farm_token_amount);

    // 5 blocks pass
    setup.b_mock.set_block_nonce(5);

    // user received no rewards, as none were deposited yet
    farm_token_nonce = setup.user_claim_rewards(farm_token_nonce, farm_token_amount);
    setup
        .b_mock
        .check_esdt_balance(&setup.user, REWARD_TOKEN_ID, &rust_biguint!(0));

    // admin deposit rewards
    setup.admin_deposit_rewards(4_000);

    // another 5 blocks pass
    setup.b_mock.set_block_nonce(10);

    // user claim rewards - received only 4_000 out of 5_000, since farm ran out of rewards
    farm_token_nonce = setup.user_claim_rewards(farm_token_nonce, farm_token_amount);
    setup
        .b_mock
        .check_esdt_balance(&setup.user, REWARD_TOKEN_ID, &rust_biguint!(4_000));

    // admin deposit more rewards
    setup.admin_deposit_rewards(50_000);

    // another 5 blocks pass
    setup.b_mock.set_block_nonce(15);

    // user claim rewards - received the full 5_000 amount
    _ = setup.user_claim_rewards(farm_token_nonce, farm_token_amount);
    setup
        .b_mock
        .check_esdt_balance(&setup.user, REWARD_TOKEN_ID, &rust_biguint!(4_000 + 5_000));
}
