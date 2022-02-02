pub mod constants;
pub mod staking_farm_with_lp_external_contracts;
pub mod staking_farm_with_lp_staking_contract_interactions;
pub mod staking_farm_with_lp_staking_contract_setup;

use constants::*;
use staking_farm_with_lp_staking_contract_interactions::*;

#[test]
fn test_all_setup() {
    let _ = FarmStakingSetup::new(
        pair::contract_obj,
        farm::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );
}

#[test]
fn test_stake_farm_proxy() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 500_500_000; // safe price of USER_TOTAL_LP_TOKENS in RIDE tokens
    let _dual_yield_token_nonce =
        setup.stake_farm_lp(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);
}

#[test]
fn test_claim_rewards_farm_proxy_full() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 500_500_000;
    let dual_yield_token_nonce_after_stake =
        setup.stake_farm_lp(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);

    setup.b_mock.set_block_nonce(BLOCK_NONCE_AFTER_SETUP + 20);

    /*
    let dual_yield_token_amount = expected_staking_token_amount;
    let _dual_yield_token_nonce_after_claim = setup.claim_rewards(
        dual_yield_token_nonce_after_stake,
        dual_yield_token_amount,
        2_777, // update value
        19_999, // update value
    );
    */
}
