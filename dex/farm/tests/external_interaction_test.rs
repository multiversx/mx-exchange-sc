#![allow(deprecated)]

mod farm_setup;

use common_structs::FarmTokenAttributes;
use farm::external_interaction::ExternalInteractionsModule;
use farm_setup::multi_user_farm_setup::{
    MultiUserFarmSetup, BOOSTED_YIELDS_PERCENTAGE, FARMING_TOKEN_ID, FARM_TOKEN_ID, MAX_PERCENTAGE,
    PER_BLOCK_REWARD_AMOUNT, REWARD_TOKEN_ID,
};
use multiversx_sc_scenario::{
    imports::TxTokenTransfer, managed_address, managed_biguint, rust_biguint, DebugApi,
};

#[test]
fn test_enter_and_claim_farm_on_behalf() {
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory::contract_obj,
        energy_update::contract_obj,
        permissions_hub::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();

    // new external user
    let external_user = farm_setup.b_mock.create_user_account(&rust_biguint!(0));

    // authorized address
    let farm_token_amount = 100_000_000;
    let farm_token_nonce = 1u64;
    let authorized_address = farm_setup.first_user.clone();

    farm_setup.whitelist_address_on_behalf(&external_user, &authorized_address);

    farm_setup.check_farm_token_supply(0);
    farm_setup.enter_farm_on_behalf(&authorized_address, &external_user, farm_token_amount, 0, 0);
    farm_setup.check_farm_token_supply(farm_token_amount);

    let block_nonce = 10u64;
    farm_setup.b_mock.set_block_nonce(block_nonce);

    // 1000 rewards per block
    let total_rewards = 1000 * block_nonce;

    // Only base rewards are given
    let base_rewards =
        total_rewards * (MAX_PERCENTAGE - BOOSTED_YIELDS_PERCENTAGE) / MAX_PERCENTAGE;
    farm_setup
        .b_mock
        .check_esdt_balance(&external_user, REWARD_TOKEN_ID, &rust_biguint!(0));
    farm_setup.claim_rewards_on_behalf(&authorized_address, farm_token_nonce, farm_token_amount);
    farm_setup.b_mock.check_esdt_balance(
        &external_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(base_rewards),
    );
}

#[test]
fn test_multiple_positions_on_behalf() {
    DebugApi::dummy();

    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory::contract_obj,
        energy_update::contract_obj,
        permissions_hub::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    let mut block_nonce = 0u64;
    farm_setup.b_mock.set_block_nonce(block_nonce);

    // new external user
    let external_user = farm_setup.b_mock.create_user_account(&rust_biguint!(0));
    farm_setup.set_user_energy(&external_user, 1_000, 1, 1);

    // authorized address
    let farm_token_amount = 100_000_000;
    let authorized_address = farm_setup.first_user.clone();

    farm_setup.whitelist_address_on_behalf(&external_user, &authorized_address);

    farm_setup.check_farm_token_supply(0);
    farm_setup.enter_farm_on_behalf(&authorized_address, &external_user, farm_token_amount, 0, 0);
    farm_setup.check_farm_token_supply(farm_token_amount);

    let block_nonce_diff = 10u64;
    block_nonce += block_nonce_diff;
    farm_setup.b_mock.set_block_nonce(block_nonce);

    // 1000 rewards per block
    let total_rewards = PER_BLOCK_REWARD_AMOUNT * block_nonce_diff;
    let base_rewards =
        total_rewards * (MAX_PERCENTAGE - BOOSTED_YIELDS_PERCENTAGE) / MAX_PERCENTAGE;
    let boosted_rewards = total_rewards * BOOSTED_YIELDS_PERCENTAGE / MAX_PERCENTAGE;

    // Only base rewards are given
    farm_setup
        .b_mock
        .check_esdt_balance(&external_user, REWARD_TOKEN_ID, &rust_biguint!(0));
    farm_setup.claim_rewards_on_behalf(&authorized_address, 1, farm_token_amount);
    farm_setup.b_mock.check_esdt_balance(
        &external_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(base_rewards),
    );

    // random tx on end of week 1, to cummulate rewards
    farm_setup.b_mock.set_block_epoch(6);
    let temp_user = farm_setup.third_user.clone();
    farm_setup.set_user_energy(&external_user, 1_000, 6, 1);
    farm_setup.set_user_energy(&temp_user, 1, 6, 1);
    farm_setup.last_farm_token_nonce = 2;
    farm_setup.enter_farm(&temp_user, 1);
    farm_setup.exit_farm(&temp_user, 3, 1);

    // advance 1 week
    block_nonce += block_nonce_diff;
    farm_setup.b_mock.set_block_nonce(block_nonce);
    farm_setup.b_mock.set_block_epoch(10);
    farm_setup.set_user_energy(&external_user, 1_000, 10, 1);

    // enter farm again for the same user (with additional payment)
    farm_setup.check_farm_token_supply(farm_token_amount);
    farm_setup.enter_farm_on_behalf(
        &authorized_address,
        &external_user,
        farm_token_amount,
        2, // nonce 2 as the user already claimed with this position
        farm_token_amount,
    );
    farm_setup.check_farm_token_supply(farm_token_amount * 2);
    farm_setup.b_mock.check_esdt_balance(
        &external_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(base_rewards + boosted_rewards),
    );

    farm_setup.claim_rewards_on_behalf(&authorized_address, 4, farm_token_amount * 2);
    farm_setup.check_farm_token_supply(farm_token_amount * 2);
    farm_setup.b_mock.check_esdt_balance(
        &external_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(total_rewards + base_rewards),
    );

    let farm_token_attributes: FarmTokenAttributes<DebugApi> = FarmTokenAttributes {
        reward_per_share: managed_biguint!(150_000_000u64),
        entering_epoch: 10u64,
        compounded_reward: managed_biguint!(0),
        current_farm_amount: managed_biguint!(farm_token_amount * 2),
        original_owner: managed_address!(&external_user),
    };

    farm_setup.b_mock.check_nft_balance(
        &authorized_address,
        FARM_TOKEN_ID,
        5,
        &rust_biguint!(farm_token_amount * 2),
        Some(&farm_token_attributes),
    );
}

#[test]
fn test_enter_and_claim_farm_on_behalf_not_whitelisted_error() {
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory::contract_obj,
        energy_update::contract_obj,
        permissions_hub::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();

    // new external user
    let external_user = farm_setup.b_mock.create_user_account(&rust_biguint!(0));

    // authorized address
    let authorized_address = farm_setup.first_user.clone();

    // Try enter without whitelist
    farm_setup
        .b_mock
        .execute_tx(
            &authorized_address,
            &farm_setup.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.enter_farm_on_behalf(managed_address!(&external_user));
            },
        )
        .assert_error(4, "Caller is not whitelisted by the user or is blacklisted");

    let farm_token_amount = 100_000_000;
    farm_setup.whitelist_address_on_behalf(&external_user, &authorized_address);
    farm_setup.enter_farm_on_behalf(&authorized_address, &external_user, farm_token_amount, 0, 0);

    // Try claim without whitelist
    farm_setup.remove_whitelist_address_on_behalf(&external_user, &authorized_address);
    farm_setup
        .b_mock
        .execute_esdt_transfer(
            &authorized_address,
            &farm_setup.farm_wrapper,
            FARM_TOKEN_ID,
            1,
            &rust_biguint!(farm_token_amount),
            |sc| {
                sc.claim_rewards_on_behalf();
            },
        )
        .assert_error(4, "Caller is not whitelisted by the user or is blacklisted");
}

#[test]
fn test_wrong_original_owner_on_behalf_validation() {
    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory::contract_obj,
        energy_update::contract_obj,
        permissions_hub::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();

    // new external users
    let external_user1 = farm_setup.b_mock.create_user_account(&rust_biguint!(0));
    let external_user2 = farm_setup.b_mock.create_user_account(&rust_biguint!(0));

    // authorized address
    let authorized_address = farm_setup.first_user.clone();

    let farm_token_amount = 100_000_000;
    farm_setup.whitelist_address_on_behalf(&external_user1, &authorized_address);
    farm_setup.whitelist_address_on_behalf(&external_user2, &authorized_address);
    farm_setup.enter_farm_on_behalf(
        &authorized_address,
        &external_user1,
        farm_token_amount,
        0,
        0,
    );
    farm_setup.enter_farm_on_behalf(
        &authorized_address,
        &external_user2,
        farm_token_amount,
        0,
        0,
    );

    // Try enter farm with wrong position
    farm_setup.b_mock.set_esdt_balance(
        &authorized_address,
        FARMING_TOKEN_ID,
        &rust_biguint!(farm_token_amount),
    );
    let mut enter_farm_payments = Vec::new();
    enter_farm_payments.push(TxTokenTransfer {
        token_identifier: FARMING_TOKEN_ID.to_vec(),
        nonce: 0,
        value: rust_biguint!(farm_token_amount),
    });
    enter_farm_payments.push(TxTokenTransfer {
        token_identifier: FARM_TOKEN_ID.to_vec(),
        nonce: 2, // external_user2 position
        value: rust_biguint!(farm_token_amount),
    });
    farm_setup
        .b_mock
        .execute_esdt_multi_transfer(
            &authorized_address,
            &farm_setup.farm_wrapper,
            &enter_farm_payments,
            |sc| {
                sc.enter_farm_on_behalf(managed_address!(&external_user1));
            },
        )
        .assert_error(4, "Provided address is not the same as the original owner");

    // Try claim with different original owners
    let mut claim_payments = Vec::new();
    claim_payments.push(TxTokenTransfer {
        token_identifier: FARM_TOKEN_ID.to_vec(),
        nonce: 1, // external_user1 position
        value: rust_biguint!(farm_token_amount),
    });
    claim_payments.push(TxTokenTransfer {
        token_identifier: FARM_TOKEN_ID.to_vec(),
        nonce: 2, // external_user2 position
        value: rust_biguint!(farm_token_amount),
    });
    farm_setup
        .b_mock
        .execute_esdt_multi_transfer(
            &authorized_address,
            &farm_setup.farm_wrapper,
            &claim_payments,
            |sc| {
                sc.claim_rewards_on_behalf();
            },
        )
        .assert_error(4, "Original owner is not the same for all payments");

    // Check enter on behalf with blacklisted address
    let blacklisted_address = farm_setup.b_mock.create_user_account(&rust_biguint!(0));
    farm_setup.whitelist_address_on_behalf(&external_user1, &blacklisted_address);
    farm_setup.blacklist_address_on_behalf(&blacklisted_address);

    farm_setup.b_mock.set_esdt_balance(
        &blacklisted_address,
        FARMING_TOKEN_ID,
        &rust_biguint!(farm_token_amount),
    );

    farm_setup
        .b_mock
        .execute_esdt_transfer(
            &blacklisted_address,
            &farm_setup.farm_wrapper,
            FARMING_TOKEN_ID,
            0,
            &rust_biguint!(farm_token_amount),
            |sc| {
                sc.enter_farm_on_behalf(managed_address!(&external_user1));
            },
        )
        .assert_error(4, "Caller is not whitelisted by the user or is blacklisted");
}

#[test]
fn test_multiple_position_claim_on_behalf_average_rps_computation() {
    DebugApi::dummy();

    let mut farm_setup = MultiUserFarmSetup::new(
        farm::contract_obj,
        energy_factory::contract_obj,
        energy_update::contract_obj,
        permissions_hub::contract_obj,
    );

    farm_setup.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);
    farm_setup.set_boosted_yields_factors();
    let mut block_nonce = 0u64;
    farm_setup.b_mock.set_block_nonce(block_nonce);

    // new external user
    let external_user = farm_setup.b_mock.create_user_account(&rust_biguint!(0));
    farm_setup.set_user_energy(&external_user, 1_000, 1, 1);

    // authorized address
    let farm_token_amount = 100_000_000;
    let authorized_address = farm_setup.first_user.clone();

    farm_setup.whitelist_address_on_behalf(&external_user, &authorized_address);

    farm_setup.check_farm_token_supply(0);
    farm_setup.enter_farm_on_behalf(
        &authorized_address,
        &external_user,
        farm_token_amount / 2,
        0,
        0,
    );
    farm_setup.check_farm_token_supply(farm_token_amount / 2);

    farm_setup.enter_farm_on_behalf(
        &authorized_address,
        &external_user,
        farm_token_amount / 2,
        0,
        0,
    );
    farm_setup.check_farm_token_supply(farm_token_amount);

    let block_nonce_diff = 10u64;
    block_nonce += block_nonce_diff;
    farm_setup.b_mock.set_block_nonce(block_nonce);

    // 1000 rewards per block
    let total_rewards = PER_BLOCK_REWARD_AMOUNT * block_nonce_diff;
    let base_rewards =
        total_rewards * (MAX_PERCENTAGE - BOOSTED_YIELDS_PERCENTAGE) / MAX_PERCENTAGE;

    // Claim first base rewards
    farm_setup
        .b_mock
        .check_esdt_balance(&external_user, REWARD_TOKEN_ID, &rust_biguint!(0));

    let mut payments = Vec::new();
    payments.push(TxTokenTransfer {
        token_identifier: FARM_TOKEN_ID.to_vec(),
        nonce: 1,
        value: rust_biguint!(farm_token_amount / 2),
    });
    payments.push(TxTokenTransfer {
        token_identifier: FARM_TOKEN_ID.to_vec(),
        nonce: 2,
        value: rust_biguint!(farm_token_amount / 2),
    });

    // First claim, without any position merged
    // Should give rewards for the first payment only and compute average RPS from both positions
    farm_setup
        .b_mock
        .execute_esdt_multi_transfer(
            &authorized_address,
            &farm_setup.farm_wrapper,
            &payments,
            |sc| {
                sc.claim_rewards_on_behalf();
            },
        )
        .assert_ok();

    farm_setup.b_mock.check_esdt_balance(
        &external_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(base_rewards / 2),
    );

    payments = Vec::new();
    payments.push(TxTokenTransfer {
        token_identifier: FARM_TOKEN_ID.to_vec(),
        nonce: 3,
        value: rust_biguint!(farm_token_amount),
    });

    // Second claim, with both position merged, in the same block
    // Should give the rest of the rewards (for the same positions)
    farm_setup
        .b_mock
        .execute_esdt_multi_transfer(
            &authorized_address,
            &farm_setup.farm_wrapper,
            &payments,
            |sc| {
                sc.claim_rewards_on_behalf();
            },
        )
        .assert_ok();

    farm_setup.b_mock.check_esdt_balance(
        &external_user,
        REWARD_TOKEN_ID,
        &rust_biguint!(base_rewards),
    );
}
