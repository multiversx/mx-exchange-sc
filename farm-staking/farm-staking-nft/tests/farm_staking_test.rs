#![allow(deprecated)]

use multiversx_sc::codec::Empty;
use multiversx_sc_scenario::{rust_biguint, whitebox_legacy::TxTokenTransfer, DebugApi};

pub mod farm_staking_setup;
use farm_staking_nft::{
    common::token_attributes::UnbondSftAttributes,
    custom_rewards::{BLOCKS_IN_YEAR, MAX_PERCENT},
};
use farm_staking_setup::*;

#[test]
fn test_farm_setup() {
    let _ = FarmStakingSetup::new(farm_staking_nft::contract_obj, energy_factory::contract_obj);
}

#[test]
fn test_enter_farm() {
    DebugApi::dummy();
    let mut farm_setup =
        FarmStakingSetup::new(farm_staking_nft::contract_obj, energy_factory::contract_obj);

    let farm_in_amount = 100_000_000;
    let farming_tokens = [
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(25_000_000),
        },
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(75_000_000),
        },
    ];
    let expected_farm_token_nonce = 1;
    farm_setup.stake_farm(
        &farming_tokens,
        &[],
        expected_farm_token_nonce,
        0,
        0,
        &farming_tokens,
    );
    farm_setup.check_farm_token_supply(farm_in_amount);
}

#[test]
fn test_unstake_farm() {
    DebugApi::dummy();
    let mut farm_setup =
        FarmStakingSetup::new(farm_staking_nft::contract_obj, energy_factory::contract_obj);

    let farm_in_amount = 100_000_000;
    let farming_tokens = [
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(25_000_000),
        },
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(75_000_000),
        },
    ];
    let expected_farm_token_nonce = 1;
    farm_setup.stake_farm(
        &farming_tokens,
        &[],
        expected_farm_token_nonce,
        0,
        0,
        &farming_tokens,
    );
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

    let expected_ride_token_balance = rust_biguint!(expected_rewards);
    farm_setup.unstake_farm(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_rewards,
        &expected_ride_token_balance,
        1,
        1,
        &UnbondSftAttributes::<DebugApi> {
            unlock_epoch: current_epoch + MIN_UNBOND_EPOCHS,
            farming_token_parts: to_managed_vec(&farming_tokens),
        },
    );
    farm_setup.check_farm_token_supply(0);
}

#[test]
fn test_claim_rewards() {
    DebugApi::dummy();
    let mut farm_setup =
        FarmStakingSetup::new(farm_staking_nft::contract_obj, energy_factory::contract_obj);

    let farm_in_amount = 100_000_000;
    let farming_tokens = [
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(25_000_000),
        },
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(75_000_000),
        },
    ];
    let expected_farm_token_nonce = 1;
    farm_setup.stake_farm(
        &farming_tokens,
        &[],
        expected_farm_token_nonce,
        0,
        0,
        &farming_tokens,
    );
    farm_setup.check_farm_token_supply(farm_in_amount);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    // value taken from the "test_unstake_farm" test
    let expected_reward_token_out = 40;
    let expected_reward_per_share = 400_000;
    farm_setup.claim_rewards(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_reward_token_out,
        &rust_biguint!(expected_reward_token_out),
        expected_farm_token_nonce + 1,
        expected_reward_per_share,
        &farming_tokens,
    );
    farm_setup.check_farm_token_supply(farm_in_amount);
}

fn steps_enter_farm_twice<FarmObjBuilder, EnergyFactoryBuilder>(
    farm_builder: FarmObjBuilder,
    energy_factory_builder: EnergyFactoryBuilder,
) -> FarmStakingSetup<FarmObjBuilder, EnergyFactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking_nft::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
{
    let mut farm_setup = FarmStakingSetup::new(farm_builder, energy_factory_builder);

    let farm_in_amount = 100_000_000;
    let farming_tokens = [
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(25_000_000),
        },
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(75_000_000),
        },
    ];
    let expected_farm_token_nonce = 1;
    farm_setup.stake_farm(
        &farming_tokens,
        &[],
        expected_farm_token_nonce,
        0,
        0,
        &farming_tokens,
    );
    farm_setup.check_farm_token_supply(farm_in_amount);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    let second_farm_in_amount = 200_000_000;
    let second_farming_tokens = [
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(50_000_000),
        },
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(150_000_000),
        },
    ];
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

    let mut all_farming_tokens = Vec::new();
    all_farming_tokens.extend_from_slice(&farming_tokens);
    all_farming_tokens.extend_from_slice(&second_farming_tokens);

    farm_setup.stake_farm(
        &second_farming_tokens,
        &prev_farm_tokens,
        expected_farm_token_nonce + 1,
        expected_reward_per_share,
        0,
        &all_farming_tokens,
    );
    farm_setup.check_farm_token_supply(total_amount);

    farm_setup
}

#[test]
fn test_enter_farm_twice() {
    DebugApi::dummy();
    let _ = steps_enter_farm_twice(farm_staking_nft::contract_obj, energy_factory::contract_obj);
}

#[test]
fn test_exit_farm_after_enter_twice() {
    DebugApi::dummy();
    let mut farm_setup =
        steps_enter_farm_twice(farm_staking_nft::contract_obj, energy_factory::contract_obj);
    let farm_in_amount = 100_000_000;
    let second_farm_in_amount = 200_000_000;

    farm_setup.set_block_epoch(8);
    farm_setup.set_block_nonce(25);

    let expected_rewards = 83;
    let all_farming_tokens = [
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(25_000_000) / 3u32,
        },
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(75_000_000) / 3u32,
        },
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(50_000_000) / 3u32,
        },
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(150_000_000) / 3u32,
        },
    ];

    farm_setup.unstake_farm(
        farm_in_amount,
        2,
        expected_rewards,
        &rust_biguint!(expected_rewards),
        1,
        1,
        &UnbondSftAttributes::<DebugApi> {
            unlock_epoch: 8 + MIN_UNBOND_EPOCHS,
            farming_token_parts: to_managed_vec(&all_farming_tokens),
        },
    );
    farm_setup.check_farm_token_supply(second_farm_in_amount);
}

#[test]
fn test_unbond() {
    DebugApi::dummy();
    let mut farm_setup =
        FarmStakingSetup::new(farm_staking_nft::contract_obj, energy_factory::contract_obj);

    let farm_in_amount = 100_000_000;
    let farming_tokens = [
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(25_000_000),
        },
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(75_000_000),
        },
    ];
    let expected_farm_token_nonce = 1;
    farm_setup.stake_farm(
        &farming_tokens,
        &[],
        expected_farm_token_nonce,
        0,
        0,
        &farming_tokens,
    );
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

    farm_setup.unstake_farm(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_rewards,
        &rust_biguint!(expected_rewards),
        1,
        1,
        &UnbondSftAttributes::<DebugApi> {
            unlock_epoch: current_epoch + MIN_UNBOND_EPOCHS,
            farming_token_parts: to_managed_vec(&farming_tokens),
        },
    );
    farm_setup.check_farm_token_supply(0);

    farm_setup.set_block_epoch(current_epoch + MIN_UNBOND_EPOCHS);
    farm_setup.unbond_farm(1, &farming_tokens);

    farm_setup.b_mock.check_nft_balance::<Empty>(
        &farm_setup.user_address,
        FARMING_TOKEN_ID,
        1,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
        None,
    );
    farm_setup.b_mock.check_nft_balance::<Empty>(
        &farm_setup.user_address,
        FARMING_TOKEN_ID,
        2,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
        None,
    );
}

#[test]
fn test_withdraw_rewards() {
    DebugApi::dummy();
    let mut farm_setup =
        FarmStakingSetup::new(farm_staking_nft::contract_obj, energy_factory::contract_obj);

    let initial_rewards_capacity = 1_000_000_000_000u64;
    farm_setup.check_rewards_capacity(initial_rewards_capacity);

    let withdraw_amount = rust_biguint!(TOTAL_REWARDS_AMOUNT);
    farm_setup.withdraw_rewards(&withdraw_amount);

    let final_rewards_capacity = 0u64;
    farm_setup.check_rewards_capacity(final_rewards_capacity);
}

#[test]
fn claim_twice_test() {
    DebugApi::dummy();
    let mut farm_setup =
        FarmStakingSetup::new(farm_staking_nft::contract_obj, energy_factory::contract_obj);

    let farm_in_amount = 100_000_000;
    let farming_tokens = [
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(25_000_000),
        },
        TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(75_000_000),
        },
    ];
    let expected_farm_token_nonce = 1;
    farm_setup.stake_farm(
        &farming_tokens,
        &[],
        expected_farm_token_nonce,
        0,
        0,
        &farming_tokens,
    );
    farm_setup.check_farm_token_supply(farm_in_amount);

    farm_setup.set_block_epoch(5);
    farm_setup.set_block_nonce(10);

    // value taken from the "test_unstake_farm" test
    let expected_reward_token_out = 40;
    let expected_reward_per_share = 400_000;
    farm_setup.claim_rewards(
        farm_in_amount,
        expected_farm_token_nonce,
        expected_reward_token_out,
        &rust_biguint!(expected_reward_token_out),
        expected_farm_token_nonce + 1,
        expected_reward_per_share,
        &farming_tokens,
    );
    farm_setup.check_farm_token_supply(farm_in_amount);

    farm_setup.set_block_epoch(10);
    farm_setup.set_block_nonce(20);

    farm_setup.claim_rewards(
        farm_in_amount,
        expected_farm_token_nonce + 1,
        expected_reward_token_out,
        &(rust_biguint!(expected_reward_token_out) * 2u32),
        expected_farm_token_nonce + 2,
        expected_reward_per_share * 2,
        &farming_tokens,
    );
    farm_setup.check_farm_token_supply(farm_in_amount);
}
