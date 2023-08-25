#![allow(deprecated)]

use multiversx_sc_scenario::{rust_biguint, whitebox_legacy::TxTokenTransfer, DebugApi};

pub mod farm_staking_setup;
use farm_staking::{
    custom_rewards::{BLOCKS_IN_YEAR, MAX_PERCENT},
    token_attributes::UnbondSftAttributes,
};
use farm_staking_setup::*;

#[test]
fn test_farm_setup() {
    let _ = FarmStakingSetup::new(farm_staking::contract_obj, energy_factory::contract_obj);
}

#[test]
fn test_enter_farm() {
    let _ = DebugApi::dummy();
    let mut farm_setup =
        FarmStakingSetup::new(farm_staking::contract_obj, energy_factory::contract_obj);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount);
}

#[test]
fn test_unstake_farm() {
    let _ = DebugApi::dummy();
    let mut farm_setup =
        FarmStakingSetup::new(farm_staking::contract_obj, energy_factory::contract_obj);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount);

    let current_block = 10;
    let current_epoch = 5;
    farm_setup.set_block_epoch(current_epoch);
    farm_setup.set_block_nonce(current_block);

    let block_diff = current_block;
    let expected_rewards_unbounded = block_diff * PER_BLOCK_REWARD_AMOUNT;

    // ~= 4 * 10 = 40
    let expected_rewards_max_apr =
        farm_in_amount * MAX_APR / MAX_PERCENT / BLOCKS_IN_YEAR * block_diff;
    let expected_rewards = core::cmp::min(expected_rewards_unbounded, expected_rewards_max_apr);
    assert_eq!(expected_rewards, 40);

    let expected_ride_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS) - farm_in_amount + expected_rewards;
    farm_setup.unstake_farm(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_rewards,
        &expected_ride_token_balance,
        &expected_ride_token_balance,
        expected_farm_token_nonce + 1,
        farm_in_amount,
        &UnbondSftAttributes {
            unlock_epoch: current_epoch + MIN_UNBOND_EPOCHS,
        },
    );
    farm_setup.check_farm_token_supply(0);
}

#[test]
fn test_claim_rewards() {
    let _ = DebugApi::dummy();
    let mut farm_setup =
        FarmStakingSetup::new(farm_staking::contract_obj, energy_factory::contract_obj);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    // value taken from the "test_unstake_farm" test
    let expected_reward_token_out = 40;
    let expected_farming_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS - farm_in_amount + expected_reward_token_out);
    let expected_reward_per_share = 400_000;
    farm_setup.claim_rewards(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_reward_token_out,
        &expected_farming_token_balance,
        &expected_farming_token_balance,
        expected_farm_token_nonce + 1,
        expected_reward_per_share,
    );
    farm_setup.check_farm_token_supply(farm_in_amount);
}

fn steps_enter_farm_twice<FarmObjBuilder, EnergyFactoryBuilder>(
    farm_builder: FarmObjBuilder,
    energy_factory_builder: EnergyFactoryBuilder,
) -> FarmStakingSetup<FarmObjBuilder, EnergyFactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
{
    let mut farm_setup = FarmStakingSetup::new(farm_builder, energy_factory_builder);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    let second_farm_in_amount = 200_000_000;
    let prev_farm_tokens = [TxTokenTransfer {
        token_identifier: FARM_TOKEN_ID.to_vec(),
        nonce: expected_farm_token_nonce,
        value: rust_biguint!(farm_in_amount),
    }];

    let total_amount = farm_in_amount + second_farm_in_amount;
    let first_reward_share = 0;
    let second_reward_share = 400_000;
    let expected_reward_per_share = (first_reward_share * farm_in_amount
        + second_reward_share * second_farm_in_amount
        + total_amount
        - 1)
        / total_amount;

    farm_setup.stake_farm(
        second_farm_in_amount,
        &prev_farm_tokens,
        expected_farm_token_nonce + 1,
        expected_reward_per_share,
        0,
    );
    farm_setup.check_farm_token_supply(total_amount);

    farm_setup
}

#[test]
fn test_enter_farm_twice() {
    let _ = DebugApi::dummy();
    let _ = steps_enter_farm_twice(farm_staking::contract_obj, energy_factory::contract_obj);
}

#[test]
fn test_exit_farm_after_enter_twice() {
    let _ = DebugApi::dummy();
    let mut farm_setup =
        steps_enter_farm_twice(farm_staking::contract_obj, energy_factory::contract_obj);
    let farm_in_amount = 100_000_000;
    let second_farm_in_amount = 200_000_000;

    farm_setup.set_block_epoch(8);
    farm_setup.set_block_nonce(25);

    let expected_rewards = 83;
    let expected_ride_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS) - farm_in_amount - second_farm_in_amount
            + expected_rewards;
    farm_setup.unstake_farm(
        farm_in_amount,
        2,
        expected_rewards,
        &expected_ride_token_balance,
        &expected_ride_token_balance,
        3,
        farm_in_amount,
        &UnbondSftAttributes {
            unlock_epoch: 8 + MIN_UNBOND_EPOCHS,
        },
    );
    farm_setup.check_farm_token_supply(second_farm_in_amount);
}

#[test]
fn test_unbond() {
    let _ = DebugApi::dummy();
    let mut farm_setup =
        FarmStakingSetup::new(farm_staking::contract_obj, energy_factory::contract_obj);

    let farm_in_amount = 100_000_000;
    let expected_farm_token_nonce = 1;
    farm_setup.stake_farm(farm_in_amount, &[], expected_farm_token_nonce, 0, 0);
    farm_setup.check_farm_token_supply(farm_in_amount);

    let current_block = 10;
    let current_epoch = 5;
    farm_setup.set_block_epoch(current_epoch);
    farm_setup.set_block_nonce(current_block);

    let block_diff = current_block;
    let expected_rewards_unbounded = block_diff * PER_BLOCK_REWARD_AMOUNT;

    // ~= 4 * 10 = 40
    let expected_rewards_max_apr =
        farm_in_amount * MAX_APR / MAX_PERCENT / BLOCKS_IN_YEAR * block_diff;
    let expected_rewards = core::cmp::min(expected_rewards_unbounded, expected_rewards_max_apr);
    assert_eq!(expected_rewards, 40);

    let expected_ride_token_balance =
        rust_biguint!(USER_TOTAL_RIDE_TOKENS) - farm_in_amount + expected_rewards;
    farm_setup.unstake_farm(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_rewards,
        &expected_ride_token_balance,
        &expected_ride_token_balance,
        expected_farm_token_nonce + 1,
        farm_in_amount,
        &UnbondSftAttributes {
            unlock_epoch: current_epoch + MIN_UNBOND_EPOCHS,
        },
    );
    farm_setup.check_farm_token_supply(0);

    farm_setup.set_block_epoch(current_epoch + MIN_UNBOND_EPOCHS);

    farm_setup.unbond_farm(
        expected_farm_token_nonce + 1,
        farm_in_amount,
        farm_in_amount,
        USER_TOTAL_RIDE_TOKENS + expected_rewards,
    );
}
