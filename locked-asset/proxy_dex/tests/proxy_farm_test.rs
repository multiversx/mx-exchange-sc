#![allow(deprecated)]

mod proxy_dex_test_setup;

use common_structs::FarmTokenAttributes;
use config::ConfigModule;
use energy_factory::{energy::EnergyModule, SimpleLockEnergy};
use energy_query::Energy;
use farm::exit_penalty::DEFAULT_PENALTY_PERCENT;
use farm::MAX_PERCENT;
use multiversx_sc::{
    codec::{multi_types::OptionalValue, Empty},
    types::{BigInt, EsdtLocalRole, EsdtTokenPayment},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    whitebox_legacy::TxTokenTransfer, DebugApi,
};
use num_traits::ToPrimitive;
use proxy_dex::{
    proxy_farm::ProxyFarmModule, proxy_pair::ProxyPairModule,
    wrapped_farm_attributes::WrappedFarmTokenAttributes,
    wrapped_farm_token_merge::WrappedFarmTokenMerge,
    wrapped_lp_attributes::WrappedLpTokenAttributes,
};
use proxy_dex_test_setup::*;
use simple_lock::locked_token::LockedTokenAttributes;

#[test]
fn farm_proxy_setup_test() {
    let _ = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
}

#[test]
fn farm_proxy_actions_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let farm_addr = setup.farm_locked_wrapper.address_ref().clone();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // check user balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(0),
        None,
    );
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );

    // check proxy balance
    setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            setup.proxy_wrapper.address_ref(),
            FARM_LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            None,
        );

    // check farm balance
    setup.b_mock.check_esdt_balance(
        setup.farm_locked_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &rust_biguint!(USER_BALANCE),
    );

    setup.b_mock.set_block_epoch(50);
    setup.b_mock.set_block_timestamp(100);

    //////////////////////////////////////////// CLAIM REWARDS /////////////////////////////////////

    // claim rewards with half position
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE / 2),
            |sc| {
                sc.claim_rewards_proxy(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // check user balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        3,
        &(rust_biguint!(PER_SECOND_REWARD_AMOUNT) * 100u32 / 2u32),
        None,
    );
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(0),
        None,
    );
    // remaining old NFT
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE / 2),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );
    // new NFT
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        2,
        &rust_biguint!(USER_BALANCE / 2),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE / 2),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 2,
                amount: managed_biguint!(USER_BALANCE / 2),
            },
        }),
    );

    //////////////////////////////////////////// MERGE TOKENS /////////////////////////////////////

    let payments = vec![
        TxTokenTransfer {
            token_identifier: WRAPPED_FARM_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(USER_BALANCE / 2),
        },
        TxTokenTransfer {
            token_identifier: WRAPPED_FARM_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(USER_BALANCE / 2),
        },
    ];
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.merge_wrapped_farm_tokens_endpoint(managed_address!(&farm_addr));
        })
        .assert_ok();

    // check user balance
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        3,
        &rust_biguint!(USER_BALANCE),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 3,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );

    // Check balance before exit farm proxy
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(0),
        None,
    );

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            3,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                let output = sc.exit_farm_proxy(managed_address!(&farm_addr), OptionalValue::None);
                let output_lp_token = output.0 .0;
                assert_eq!(output_lp_token.token_nonce, 1);
                assert_eq!(output_lp_token.amount, USER_BALANCE);
            },
        )
        .assert_ok();

    // Check balance after exit farm proxy
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(MEX_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: LOCK_OPTIONS[0],
        }),
    );
}

#[test]
fn farm_with_wrapped_lp_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );

    setup
        .b_mock
        .execute_tx(
            &setup.owner,
            &setup.farm_locked_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.farming_token_id().set(&managed_token_id!(LP_TOKEN_ID));

                // set produce rewards to false for easier calculation
                sc.produce_rewards_enabled().set(false);
            },
        )
        .assert_ok();

    setup.b_mock.set_esdt_local_roles(
        setup.farm_locked_wrapper.address_ref(),
        LP_TOKEN_ID,
        &[EsdtLocalRole::Burn],
    );

    let first_user = setup.first_user.clone();
    let locked_token_amount = rust_biguint!(1_000_000_000);
    let other_token_amount = rust_biguint!(500_000_000);
    let expected_lp_token_amount = rust_biguint!(499_999_000);

    // set the price to 1 EGLD = 2 MEX
    let payments = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount.clone(),
        },
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: other_token_amount.clone(),
        },
    ];

    // add liquidity
    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount.to_u64().unwrap()),
                managed_biguint!(other_token_amount.to_u64().unwrap()),
            );
        })
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_LP_TOKEN_ID,
        1,
        &expected_lp_token_amount,
        Some(&WrappedLpTokenAttributes::<DebugApi> {
            locked_tokens: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(locked_token_amount.to_u64().unwrap()),
            },
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            lp_token_amount: managed_biguint!(expected_lp_token_amount.to_u64().unwrap()),
        }),
    );

    let block_epoch = 1u64;
    let user_balance = USER_BALANCE;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let unlock_epoch = LOCK_OPTIONS[0];
            let lock_epochs = unlock_epoch - block_epoch;
            let expected_energy_amount =
                BigInt::from((user_balance) as i64) * BigInt::from(lock_epochs as i64);
            let expected_energy = Energy::new(
                expected_energy_amount,
                block_epoch,
                managed_biguint!(user_balance),
            );
            let actual_energy =
                sc.get_updated_energy_entry_for_user(&managed_address!(&first_user));
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();

    let farm_locked_addr = setup.farm_locked_wrapper.address_ref().clone();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    let mut current_epoch = 5;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            1,
            &expected_lp_token_amount,
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    let expected_energy = rust_biguint!(LOCK_OPTIONS[0] - current_epoch) * USER_BALANCE;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let managed_result = sc.get_energy_amount_for_user(managed_address!(&first_user));
            let result = to_rust_biguint(managed_result);
            assert_eq!(result, expected_energy);
        })
        .assert_ok();

    // check user balance
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        1,
        &expected_lp_token_amount,
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(WRAPPED_LP_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(expected_lp_token_amount.to_u64().unwrap()),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(expected_lp_token_amount.to_u64().unwrap()),
            },
        }),
    );

    // check proxy balance
    setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            setup.proxy_wrapper.address_ref(),
            FARM_LOCKED_TOKEN_ID,
            1,
            &expected_lp_token_amount,
            None,
        );

    // check farm balance
    setup.b_mock.check_esdt_balance(
        setup.farm_locked_wrapper.address_ref(),
        LP_TOKEN_ID,
        &expected_lp_token_amount,
    );

    current_epoch += 1; // applies penalty on exit
    setup.b_mock.set_block_epoch(current_epoch);
    setup.b_mock.set_block_timestamp(100);

    ////////////////////////////////////////////// EXIT FARM /////////////////////////////////////
    // exit with partial amount
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            1,
            &(expected_lp_token_amount.clone() / rust_biguint!(2)),
            |sc| {
                sc.exit_farm_proxy(managed_address!(&farm_locked_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    let penalty_amount = &expected_lp_token_amount / 2u64 * DEFAULT_PENALTY_PERCENT / MAX_PERCENT;

    // check proxy received only part of LP tokens back
    setup.b_mock.check_esdt_balance(
        setup.proxy_wrapper.address_ref(),
        LP_TOKEN_ID,
        &(&expected_lp_token_amount / 2u64 - &penalty_amount),
    );

    // check user received half of the farm tokens back, and another new wrapped LP token
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        1,
        &(&expected_lp_token_amount / 2u64),
        None,
    );
    // user received 495_000_000 locked tokens in the new token
    // less than half of the original 1_000_000_000, i.e. 500_000_000
    let locked_token_after_exit = rust_biguint!(495_000_000);
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_LP_TOKEN_ID,
        2,
        &(&expected_lp_token_amount / 2u64 - &penalty_amount),
        Some(&WrappedLpTokenAttributes::<DebugApi> {
            locked_tokens: EsdtTokenPayment::new(
                managed_token_id!(LOCKED_TOKEN_ID),
                1,
                managed_biguint!(locked_token_after_exit.to_u64().unwrap()),
            ),
            lp_token_id: managed_token_id!(LP_TOKEN_ID),
            lp_token_amount: managed_biguint!(
                expected_lp_token_amount.to_u64().unwrap() / 2u64
                    - penalty_amount.to_u64().unwrap()
            ),
        }),
    );

    // check user's energy
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let new_user_balance = managed_biguint!(USER_BALANCE)
                - locked_token_amount.to_u64().unwrap() / 2u64
                + locked_token_after_exit.to_u64().unwrap();
            let expected_energy_amount =
                managed_biguint!(LOCK_OPTIONS[0] - current_epoch) * &new_user_balance;

            let expected_energy = Energy::new(
                BigInt::from(expected_energy_amount),
                current_epoch,
                new_user_balance,
            );
            let actual_energy =
                sc.get_updated_energy_entry_for_user(&managed_address!(&first_user));

            assert_eq!(actual_energy, expected_energy);
        })
        .assert_ok();
}

#[test]
fn farm_proxy_claim_energy_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let farm_locked_addr = setup.farm_locked_wrapper.address_ref().clone();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    let current_epoch = 5;
    setup.b_mock.set_block_epoch(current_epoch);

    let expected_energy = rust_biguint!(LOCK_OPTIONS[0] - current_epoch) * USER_BALANCE;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let managed_result = sc.get_energy_amount_for_user(managed_address!(&first_user));
            let result = to_rust_biguint(managed_result);
            assert_eq!(result, expected_energy);
        })
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    let expected_energy = rust_biguint!(LOCK_OPTIONS[0] - current_epoch) * USER_BALANCE;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let managed_result = sc.get_energy_amount_for_user(managed_address!(&first_user));
            let result = to_rust_biguint(managed_result);
            assert_eq!(result, expected_energy);
        })
        .assert_ok();

    // check user balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(0),
        None,
    );
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );

    // check proxy balance
    setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            setup.proxy_wrapper.address_ref(),
            FARM_LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            None,
        );

    // check farm balance
    setup.b_mock.check_esdt_balance(
        setup.farm_locked_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &rust_biguint!(USER_BALANCE),
    );

    setup.b_mock.set_block_timestamp(100);

    //////////////////////////////////////////// CLAIM REWARDS /////////////////////////////////////

    // claim rewards
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.claim_rewards_proxy(managed_address!(&farm_locked_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // check user balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &(rust_biguint!(PER_SECOND_REWARD_AMOUNT) * 100u32),
        None,
    );

    let new_user_balance = USER_BALANCE + rust_biguint!(PER_SECOND_REWARD_AMOUNT) * 100u32;
    let expected_energy = rust_biguint!(LOCK_OPTIONS[0] - current_epoch) * new_user_balance;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let managed_result = sc.get_energy_amount_for_user(managed_address!(&first_user));
            let result = to_rust_biguint(managed_result);
            assert_eq!(result, expected_energy);
        })
        .assert_ok();
}

#[test]
fn farm_proxy_partial_exit_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let farm_locked_addr = setup.farm_locked_wrapper.address_ref().clone();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    let mut current_epoch = 5;
    setup.b_mock.set_block_epoch(current_epoch);

    let expected_energy = rust_biguint!(LOCK_OPTIONS[0] - current_epoch) * USER_BALANCE;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let managed_result = sc.get_energy_amount_for_user(managed_address!(&first_user));
            let result = to_rust_biguint(managed_result);
            assert_eq!(result, expected_energy);
        })
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    let expected_energy = rust_biguint!(LOCK_OPTIONS[0] - current_epoch) * USER_BALANCE;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let managed_result = sc.get_energy_amount_for_user(managed_address!(&first_user));
            let result = to_rust_biguint(managed_result);
            assert_eq!(result, expected_energy);
        })
        .assert_ok();

    // check user balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(0),
        None,
    );
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );

    // check proxy balance
    setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            setup.proxy_wrapper.address_ref(),
            FARM_LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            None,
        );

    // check farm balance
    setup.b_mock.check_esdt_balance(
        setup.farm_locked_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &rust_biguint!(USER_BALANCE),
    );

    current_epoch += 3; // does not apply penalty on exit
    setup.b_mock.set_block_epoch(current_epoch);
    setup.b_mock.set_block_timestamp(100);

    //////////////////////////////////////////// PARTIAL EXIT /////////////////////////////////////

    // partial exit farm with 50%
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE / 2),
            |sc| {
                sc.exit_farm_proxy(managed_address!(&farm_locked_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // check user balance - base rewards for partial position (50%) + remaining balance (50%)
    // rewards for the full position only applies for the boosted rewards
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &(rust_biguint!(PER_SECOND_REWARD_AMOUNT * 100 / 2 + USER_BALANCE / 2)),
        None,
    );

    let new_user_balance = USER_BALANCE + rust_biguint!(PER_SECOND_REWARD_AMOUNT) * 100u32 / 2u32;
    let expected_energy = rust_biguint!(LOCK_OPTIONS[0] - current_epoch) * new_user_balance;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let managed_result = sc.get_energy_amount_for_user(managed_address!(&first_user));
            let result = to_rust_biguint(managed_result);
            assert_eq!(result, expected_energy);
        })
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE / 2),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );

    // check proxy balance
    setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            setup.proxy_wrapper.address_ref(),
            FARM_LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE / 2),
            None,
        );

    // check farm balance
    setup.b_mock.check_esdt_balance(
        setup.farm_locked_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &rust_biguint!(USER_BALANCE / 2),
    );
}

#[test]
fn farm_proxy_partial_exit_with_penalty_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let farm_locked_addr = setup.farm_locked_wrapper.address_ref().clone();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    let mut current_epoch = 5;
    setup.b_mock.set_block_epoch(current_epoch);

    let expected_energy = rust_biguint!(LOCK_OPTIONS[0] - current_epoch) * USER_BALANCE;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let managed_result = sc.get_energy_amount_for_user(managed_address!(&first_user));
            let result = to_rust_biguint(managed_result);
            assert_eq!(result, expected_energy);
        })
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    let expected_energy = rust_biguint!(LOCK_OPTIONS[0] - current_epoch) * USER_BALANCE;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let managed_result = sc.get_energy_amount_for_user(managed_address!(&first_user));
            let result = to_rust_biguint(managed_result);
            assert_eq!(result, expected_energy);
        })
        .assert_ok();

    // check user balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(0),
        None,
    );
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );

    // check proxy balance
    setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            setup.proxy_wrapper.address_ref(),
            FARM_LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            None,
        );

    // check farm balance
    setup.b_mock.check_esdt_balance(
        setup.farm_locked_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &rust_biguint!(USER_BALANCE),
    );

    current_epoch += 1; // applies penalty on exit
    setup.b_mock.set_block_epoch(current_epoch);
    setup.b_mock.set_block_timestamp(100);

    //////////////////////////////////////////// PARTIAL EXIT /////////////////////////////////////

    // partial exit farm with 50%
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE / 2),
            |sc| {
                sc.exit_farm_proxy(managed_address!(&farm_locked_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // check user balance - base rewards for partial position (50%) + (remaining balance (50%) - applied penalty for early exit (1%))
    // rewards for the full position only applies for the boosted rewards
    let tokens_received_at_exit = rust_biguint!(PER_SECOND_REWARD_AMOUNT * 100 / 2)
        + rust_biguint!(USER_BALANCE / 2)
        - rust_biguint!(USER_BALANCE / 2) * DEFAULT_PENALTY_PERCENT / MAX_PERCENT;

    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &tokens_received_at_exit,
        None,
    );

    let new_user_balance = rust_biguint!(USER_BALANCE / 2) + tokens_received_at_exit;
    let expected_energy = rust_biguint!(LOCK_OPTIONS[0] - current_epoch) * new_user_balance;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let managed_result = sc.get_energy_amount_for_user(managed_address!(&first_user));
            let result = to_rust_biguint(managed_result);
            assert_eq!(result, expected_energy);
        })
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE / 2),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );

    // check proxy balance
    setup
        .b_mock
        .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
            setup.proxy_wrapper.address_ref(),
            FARM_LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE / 2),
            None,
        );

    // check farm balance
    setup.b_mock.check_esdt_balance(
        setup.farm_locked_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &rust_biguint!(USER_BALANCE / 2),
    );
}

#[test]
fn different_farm_locked_token_nonce_merging_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let farm_addr = setup.farm_locked_wrapper.address_ref().clone();
    let user_balance = rust_biguint!(USER_BALANCE);
    setup
        .b_mock
        .set_esdt_balance(&first_user, MEX_TOKEN_ID, &user_balance);

    // users lock tokens
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.simple_lock_wrapper,
            MEX_TOKEN_ID,
            0,
            &user_balance,
            |sc| {
                let user_payment = sc.lock_tokens_endpoint(LOCK_OPTIONS[1], OptionalValue::None);
                assert_eq!(user_payment.token_nonce, 2);
                assert_eq!(user_payment.amount, managed_biguint!(USER_BALANCE));
            },
        )
        .assert_ok();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            2,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );

    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        2,
        &rust_biguint!(USER_BALANCE),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 2,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 2,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );

    //////////////////////////////////////////// MERGE TOKENS /////////////////////////////////////

    let payments = vec![
        TxTokenTransfer {
            token_identifier: WRAPPED_FARM_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(USER_BALANCE),
        },
        TxTokenTransfer {
            token_identifier: WRAPPED_FARM_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(USER_BALANCE),
        },
    ];
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.merge_wrapped_farm_tokens_endpoint(managed_address!(&farm_addr));
        })
        .assert_ok();

    // check user balance
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        3,
        &rust_biguint!(USER_BALANCE * 2),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 3,
                amount: managed_biguint!(USER_BALANCE * 2),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 3,
                amount: managed_biguint!(USER_BALANCE * 2),
            },
        }),
    );

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            3,
            &rust_biguint!(USER_BALANCE * 2),
            |sc| {
                sc.exit_farm_proxy(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    let expected_unlock_epoch = (LOCK_OPTIONS[0] + LOCK_OPTIONS[1]) / 2;
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        3,
        &rust_biguint!(1_980_000_000_000_000_000u64),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(MEX_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: expected_unlock_epoch,
        }),
    );
}

#[test]
fn total_farm_mechanism_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let farm_addr = setup.farm_locked_wrapper.address_ref().clone();
    let user_balance = rust_biguint!(USER_BALANCE);
    setup
        .b_mock
        .set_esdt_balance(&first_user, MEX_TOKEN_ID, &user_balance);

    // users lock tokens
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.simple_lock_wrapper,
            MEX_TOKEN_ID,
            0,
            &user_balance,
            |sc| {
                let user_payment = sc.lock_tokens_endpoint(LOCK_OPTIONS[1], OptionalValue::None);
                assert_eq!(user_payment.token_nonce, 2);
                assert_eq!(user_payment.amount, managed_biguint!(USER_BALANCE));
            },
        )
        .assert_ok();

    let mut block_epoch = 1;
    setup.b_mock.set_block_epoch(block_epoch);

    // Check total farm position
    setup
        .b_mock
        .execute_query(&setup.farm_locked_wrapper, |sc| {
            let first_user_total_farm_position = sc
                .user_total_farm_position(&managed_address!(&first_user))
                .get();
            assert_eq!(first_user_total_farm_position, managed_biguint!(0));
        })
        .assert_ok();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // Check total farm position
    setup
        .b_mock
        .execute_query(&setup.farm_locked_wrapper, |sc| {
            let first_user_total_farm_position = sc
                .user_total_farm_position(&managed_address!(&first_user))
                .get();
            assert_eq!(
                first_user_total_farm_position,
                managed_biguint!(USER_BALANCE)
            );
        })
        .assert_ok();

    block_epoch += 7;
    setup.b_mock.set_block_epoch(block_epoch);

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            2,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // Check total farm position
    setup
        .b_mock
        .execute_query(&setup.farm_locked_wrapper, |sc| {
            let first_user_total_farm_position = sc
                .user_total_farm_position(&managed_address!(&first_user))
                .get();
            assert_eq!(
                first_user_total_farm_position,
                managed_biguint!(USER_BALANCE * 2)
            );
        })
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );

    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        2,
        &rust_biguint!(USER_BALANCE),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 2,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 2,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );

    //////////////////////////////////////////// CLAIM REWARDS /////////////////////////////////////

    // claim rewards
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.claim_rewards_proxy(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // Check total farm position
    setup
        .b_mock
        .execute_query(&setup.farm_locked_wrapper, |sc| {
            let first_user_total_farm_position = sc
                .user_total_farm_position(&managed_address!(&first_user))
                .get();
            assert_eq!(
                first_user_total_farm_position,
                managed_biguint!(USER_BALANCE * 2)
            );
        })
        .assert_ok();

    // check user balance
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        2,
        &rust_biguint!(0),
        None,
    );
    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(0),
        None,
    );

    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        3,
        &rust_biguint!(USER_BALANCE),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 1,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 3,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );
    // new NFT
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        2,
        &rust_biguint!(USER_BALANCE),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(LOCKED_TOKEN_ID),
                token_nonce: 2,
                amount: managed_biguint!(USER_BALANCE),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 2,
                amount: managed_biguint!(USER_BALANCE),
            },
        }),
    );
}

#[test]
fn increase_proxy_farm_lkmex_energy() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let farm_addr = setup.farm_locked_wrapper.address_ref().clone();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    let block_epoch = 1;

    // check user energy before
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let lock_epochs = LOCK_OPTIONS[0] - block_epoch;
            let expected_energy_amount =
                BigInt::from((USER_BALANCE) as i64) * BigInt::from(lock_epochs as i64);
            let expected_energy = Energy::new(
                expected_energy_amount,
                block_epoch,
                managed_biguint!(USER_BALANCE),
            );
            let actual_energy =
                sc.get_updated_energy_entry_for_user(&managed_address!(&first_user));
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();

    //////////////////////////////////////////// INCREASE ENERGY /////////////////////////////////////
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.increase_proxy_farm_token_energy_endpoint(LOCK_OPTIONS[1]);
            },
        )
        .assert_ok();

    // check user energy after
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let lock_epochs = LOCK_OPTIONS[1] - block_epoch;
            let expected_energy_amount =
                BigInt::from((USER_BALANCE) as i64) * BigInt::from(lock_epochs as i64);
            let expected_energy = Energy::new(
                expected_energy_amount,
                block_epoch,
                managed_biguint!(USER_BALANCE),
            );
            let actual_energy =
                sc.get_updated_energy_entry_for_user(&managed_address!(&first_user));
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();
}

#[test]
fn increase_proxy_farm_proxy_lp_energy() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );

    setup
        .b_mock
        .execute_tx(
            &setup.owner,
            &setup.farm_locked_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.farming_token_id().set(&managed_token_id!(LP_TOKEN_ID));

                // set produce rewards to false for easier calculation
                sc.produce_rewards_enabled().set(false);
            },
        )
        .assert_ok();

    setup.b_mock.set_esdt_local_roles(
        setup.farm_locked_wrapper.address_ref(),
        LP_TOKEN_ID,
        &[EsdtLocalRole::Burn],
    );

    let first_user = setup.first_user.clone();
    let locked_token_amount = rust_biguint!(1_000_000_000);
    let other_token_amount = rust_biguint!(500_000_000);
    let expected_lp_token_amount = rust_biguint!(499_999_000);

    // set the price to 1 EGLD = 2 MEX
    let payments = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount.clone(),
        },
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: other_token_amount.clone(),
        },
    ];

    // add liquidity twice, to have 2 nonces
    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount.to_u64().unwrap() / 2),
                managed_biguint!(other_token_amount.to_u64().unwrap() / 2),
            );
        })
        .assert_ok();

    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount.to_u64().unwrap() / 2),
                managed_biguint!(other_token_amount.to_u64().unwrap() / 2),
            );
        })
        .assert_ok();

    let block_epoch = 1u64;
    let user_balance = USER_BALANCE;

    // check energy before
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let unlock_epoch = LOCK_OPTIONS[0];
            let lock_epochs = unlock_epoch - block_epoch;
            let expected_energy_amount =
                BigInt::from((user_balance) as i64) * BigInt::from(lock_epochs as i64);
            let expected_energy = Energy::new(
                expected_energy_amount,
                block_epoch,
                managed_biguint!(user_balance),
            );
            let actual_energy =
                sc.get_updated_energy_entry_for_user(&managed_address!(&first_user));
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();

    let farm_locked_addr = setup.farm_locked_wrapper.address_ref().clone();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    // Enter multiple times, to distribute the nonces
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            1,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            2,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            1,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            2,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    //////////////////////////////////////////// INCREASE ENERGY /////////////////////////////////////
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            4,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.increase_proxy_farm_token_energy_endpoint(LOCK_OPTIONS[1]);
            },
        )
        .assert_ok();

    // check old tokens were burned
    setup
        .b_mock
        .check_nft_balance::<WrappedFarmTokenAttributes<DebugApi>>(
            setup.proxy_wrapper.address_ref(),
            WRAPPED_FARM_TOKEN_ID,
            4,
            &rust_biguint!(0u64),
            None,
        );

    // check energy after
    // lp tokens recharged = total tokens / 4 - 500
    let user_locked_tokens_in_lp = locked_token_amount.to_u64().unwrap() / 4 - 500;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let first_lock_epochs = LOCK_OPTIONS[1] - block_epoch;
            let second_lock_epochs = LOCK_OPTIONS[0] - block_epoch;
            let expected_energy_amount = BigInt::from((user_locked_tokens_in_lp) as i64)
                * BigInt::from(first_lock_epochs as i64)
                + BigInt::from((USER_BALANCE - user_locked_tokens_in_lp) as i64)
                    * BigInt::from(second_lock_epochs as i64);
            let expected_energy = Energy::new(
                expected_energy_amount,
                block_epoch,
                managed_biguint!(USER_BALANCE),
            );
            let actual_energy =
                sc.get_updated_energy_entry_for_user(&managed_address!(&first_user));
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();

    // check user token after increase energy
    // new farm token was created
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        5,
        &(&expected_lp_token_amount / &rust_biguint!(4u64)),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(WRAPPED_LP_TOKEN_ID),
                token_nonce: 3,
                amount: managed_biguint!(expected_lp_token_amount.to_u64().unwrap() / 4u64),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 4,
                amount: managed_biguint!(expected_lp_token_amount.to_u64().unwrap() / 4u64),
            },
        }),
    );
}

#[test]
fn increase_proxy_farm_proxy_lp_energy_unlocked_tokens() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );

    setup
        .b_mock
        .execute_tx(
            &setup.owner,
            &setup.farm_locked_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.farming_token_id().set(&managed_token_id!(LP_TOKEN_ID));

                // set produce rewards to false for easier calculation
                sc.produce_rewards_enabled().set(false);
            },
        )
        .assert_ok();

    setup.b_mock.set_esdt_local_roles(
        setup.farm_locked_wrapper.address_ref(),
        LP_TOKEN_ID,
        &[EsdtLocalRole::Burn],
    );

    let first_user = setup.first_user.clone();
    let locked_token_amount = rust_biguint!(1_000_000_000);
    let other_token_amount = rust_biguint!(500_000_000);
    let expected_lp_token_amount = rust_biguint!(499_999_000);

    // set the price to 1 EGLD = 2 MEX
    let payments = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount.clone(),
        },
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: other_token_amount.clone(),
        },
    ];

    // add liquidity twice, to have 2 nonces
    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount.to_u64().unwrap() / 2),
                managed_biguint!(other_token_amount.to_u64().unwrap() / 2),
            );
        })
        .assert_ok();

    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount.to_u64().unwrap() / 2),
                managed_biguint!(other_token_amount.to_u64().unwrap() / 2),
            );
        })
        .assert_ok();

    let mut block_epoch = 1u64;
    let user_balance = USER_BALANCE;

    // check energy before
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let unlock_epoch = LOCK_OPTIONS[0];
            let lock_epochs = unlock_epoch - block_epoch;
            let expected_energy_amount =
                BigInt::from((user_balance) as i64) * BigInt::from(lock_epochs as i64);
            let expected_energy = Energy::new(
                expected_energy_amount,
                block_epoch,
                managed_biguint!(user_balance),
            );
            let actual_energy =
                sc.get_updated_energy_entry_for_user(&managed_address!(&first_user));
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();

    let farm_locked_addr = setup.farm_locked_wrapper.address_ref().clone();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    // Enter multiple times, to distribute the nonces
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            1,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            2,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            1,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            2,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    ////////////////////////////// Wait for tokens to unlock /////////////////////////////////////
    block_epoch += LOCK_OPTIONS[0];
    setup.b_mock.set_block_epoch(block_epoch);

    //////////////////////////////////////////// INCREASE ENERGY /////////////////////////////////////
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            4,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.increase_proxy_farm_token_energy_endpoint(LOCK_OPTIONS[1]);
            },
        )
        .assert_ok();

    // check old tokens were burned
    setup
        .b_mock
        .check_nft_balance::<WrappedFarmTokenAttributes<DebugApi>>(
            setup.proxy_wrapper.address_ref(),
            WRAPPED_FARM_TOKEN_ID,
            4,
            &rust_biguint!(0u64),
            None,
        );

    // check energy after
    // lp tokens recharged = total tokens / 4 - 500
    let user_locked_tokens_in_lp = locked_token_amount.to_u64().unwrap() / 4 - 500;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let first_lock_epochs = LOCK_OPTIONS[1] - 1u64;
            let second_lock_epochs =
                BigInt::from(LOCK_OPTIONS[0] as i64) - BigInt::from(block_epoch as i64);
            let expected_energy_amount = BigInt::from((user_locked_tokens_in_lp) as i64)
                * BigInt::from(first_lock_epochs as i64)
                + BigInt::from((USER_BALANCE - user_locked_tokens_in_lp) as i64)
                    * second_lock_epochs;
            let expected_energy = Energy::new(
                expected_energy_amount,
                block_epoch,
                managed_biguint!(USER_BALANCE),
            );
            let actual_energy =
                sc.get_updated_energy_entry_for_user(&managed_address!(&first_user));
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();

    // check user token after increase energy
    // new farm token was created
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        5,
        &(&expected_lp_token_amount / &rust_biguint!(4u64)),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(WRAPPED_LP_TOKEN_ID),
                token_nonce: 3,
                amount: managed_biguint!(expected_lp_token_amount.to_u64().unwrap() / 4u64),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 4,
                amount: managed_biguint!(expected_lp_token_amount.to_u64().unwrap() / 4u64),
            },
        }),
    );
}

#[test]
fn increase_proxy_farm_proxy_lp_energy_partially_unlocked_tokens() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );

    setup
        .b_mock
        .execute_tx(
            &setup.owner,
            &setup.farm_locked_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.farming_token_id().set(&managed_token_id!(LP_TOKEN_ID));

                // set produce rewards to false for easier calculation
                sc.produce_rewards_enabled().set(false);
            },
        )
        .assert_ok();

    setup.b_mock.set_esdt_local_roles(
        setup.farm_locked_wrapper.address_ref(),
        LP_TOKEN_ID,
        &[EsdtLocalRole::Burn],
    );

    let first_user = setup.first_user.clone();
    let locked_token_amount = rust_biguint!(1_000_000_000);
    let other_token_amount = rust_biguint!(500_000_000);
    let expected_lp_token_amount = rust_biguint!(499_999_000);

    // set the price to 1 EGLD = 2 MEX
    let payments = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: locked_token_amount.clone(),
        },
        TxTokenTransfer {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: other_token_amount.clone(),
        },
    ];

    // add liquidity twice, to have 2 nonces
    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount.to_u64().unwrap() / 2),
                managed_biguint!(other_token_amount.to_u64().unwrap() / 2),
            );
        })
        .assert_ok();

    let pair_addr = setup.pair_wrapper.address_ref().clone();
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.add_liquidity_proxy(
                managed_address!(&pair_addr),
                managed_biguint!(locked_token_amount.to_u64().unwrap() / 2),
                managed_biguint!(other_token_amount.to_u64().unwrap() / 2),
            );
        })
        .assert_ok();

    let block_epoch = 1u64;
    let user_balance = USER_BALANCE;

    // check energy before
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let unlock_epoch = LOCK_OPTIONS[0];
            let lock_epochs = unlock_epoch - block_epoch;
            let expected_energy_amount =
                BigInt::from((user_balance) as i64) * BigInt::from(lock_epochs as i64);
            let expected_energy = Energy::new(
                expected_energy_amount,
                block_epoch,
                managed_biguint!(user_balance),
            );
            let actual_energy =
                sc.get_updated_energy_entry_for_user(&managed_address!(&first_user));
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();

    let farm_locked_addr = setup.farm_locked_wrapper.address_ref().clone();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    // Enter multiple times, to distribute the nonces
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            1,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            2,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            1,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_LP_TOKEN_ID,
            2,
            &(&expected_lp_token_amount / &rust_biguint!(4u64)),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_locked_addr),
                    OptionalValue::None,
                );
            },
        )
        .assert_ok();

    //////////////////////////////////////////// INCREASE ENERGY /////////////////////////////////////
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            4,
            &(&expected_lp_token_amount / &rust_biguint!(8u64)),
            |sc| {
                sc.increase_proxy_farm_token_energy_endpoint(LOCK_OPTIONS[1]);
            },
        )
        .assert_ok();

    // check old tokens were burned
    setup
        .b_mock
        .check_nft_balance::<WrappedFarmTokenAttributes<DebugApi>>(
            setup.proxy_wrapper.address_ref(),
            WRAPPED_FARM_TOKEN_ID,
            4,
            &rust_biguint!(0u64),
            None,
        );

    // check energy after
    // lp tokens recharged = total tokens / 4 - 500
    let user_locked_tokens_in_lp = locked_token_amount.to_u64().unwrap() / 4 - 500;
    setup
        .b_mock
        .execute_query(&setup.simple_lock_wrapper, |sc| {
            let first_lock_epochs = LOCK_OPTIONS[1] - 1u64;
            let second_lock_epochs =
                BigInt::from(LOCK_OPTIONS[0] as i64) - BigInt::from(block_epoch as i64);
            let expected_energy_amount = BigInt::from((user_locked_tokens_in_lp / 2) as i64)
                * BigInt::from(first_lock_epochs as i64)
                + BigInt::from((USER_BALANCE) as i64) * second_lock_epochs.clone()
                - BigInt::from((user_locked_tokens_in_lp / 2u64) as i64) * second_lock_epochs;

            let expected_energy = Energy::new(
                expected_energy_amount,
                block_epoch,
                managed_biguint!(USER_BALANCE),
            );
            let actual_energy =
                sc.get_updated_energy_entry_for_user(&managed_address!(&first_user));
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();

    // check user token after increase energy
    // new farm token was created
    setup.b_mock.check_nft_balance(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        5,
        &(&expected_lp_token_amount / &rust_biguint!(8u64)),
        Some(&WrappedFarmTokenAttributes::<DebugApi> {
            proxy_farming_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(WRAPPED_LP_TOKEN_ID),
                token_nonce: 3,
                amount: managed_biguint!(expected_lp_token_amount.to_u64().unwrap() / 8u64),
            },
            farm_token: EsdtTokenPayment {
                token_identifier: managed_token_id!(FARM_LOCKED_TOKEN_ID),
                token_nonce: 4,
                amount: managed_biguint!(expected_lp_token_amount.to_u64().unwrap() / 8u64),
            },
        }),
    );
}

#[test]
fn original_caller_negative_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let farm_addr = setup.farm_locked_wrapper.address_ref().clone();
    let user_balance = rust_biguint!(USER_BALANCE);
    setup
        .b_mock
        .set_esdt_balance(&first_user, MEX_TOKEN_ID, &user_balance);

    // users lock tokens
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.simple_lock_wrapper,
            MEX_TOKEN_ID,
            0,
            &user_balance,
            |sc| {
                let user_payment = sc.lock_tokens_endpoint(LOCK_OPTIONS[1], OptionalValue::None);
                assert_eq!(user_payment.token_nonce, 2);
                assert_eq!(user_payment.amount, managed_biguint!(USER_BALANCE));
            },
        )
        .assert_ok();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(
                    managed_address!(&farm_addr),
                    Some(managed_address!(&first_user)).into(),
                );
            },
        )
        .assert_error(4, "Item not whitelisted");

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // claim rewards with half position
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE / 2),
            |sc| {
                sc.claim_rewards_proxy(
                    managed_address!(&farm_addr),
                    Some(managed_address!(&first_user)).into(),
                );
            },
        )
        .assert_error(4, "Item not whitelisted");

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                let output = sc.exit_farm_proxy(
                    managed_address!(&farm_addr),
                    Some(managed_address!(&first_user)).into(),
                );
                let output_lp_token = output.0 .0;
                assert_eq!(output_lp_token.token_nonce, 1);
                assert_eq!(output_lp_token.amount, USER_BALANCE);
            },
        )
        .assert_error(4, "Item not whitelisted");
}

#[test]
fn total_farm_position_migration_through_proxy_dex_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let farm_addr = setup.farm_locked_wrapper.address_ref().clone();
    let user_balance = rust_biguint!(USER_BALANCE);
    setup
        .b_mock
        .set_esdt_balance(&first_user, MEX_TOKEN_ID, &user_balance);

    // user locks tokens
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.simple_lock_wrapper,
            MEX_TOKEN_ID,
            0,
            &user_balance,
            |sc| {
                let user_payment = sc.lock_tokens_endpoint(LOCK_OPTIONS[1], OptionalValue::None);
                assert_eq!(user_payment.token_nonce, 2);
                assert_eq!(user_payment.amount, managed_biguint!(USER_BALANCE));
            },
        )
        .assert_ok();

    // User enter farm before migration
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // Simulate contract upgrade - total farm position is reset and migration nonce set
    setup
        .b_mock
        .execute_tx(
            &setup.owner,
            &setup.farm_locked_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.user_total_farm_position(&managed_address!(&first_user))
                    .set(managed_biguint!(0u64));
                sc.farm_position_migration_nonce().set(2u64);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_query(&setup.farm_locked_wrapper, |sc| {
            let first_user_total_farm_position = sc
                .user_total_farm_position(&managed_address!(&first_user))
                .get();
            assert_eq!(first_user_total_farm_position, managed_biguint!(0));
        })
        .assert_ok();

    // User enters farm again after migration
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            2,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_query(&setup.farm_locked_wrapper, |sc| {
            let first_user_total_farm_position = sc
                .user_total_farm_position(&managed_address!(&first_user))
                .get();
            assert_eq!(
                first_user_total_farm_position,
                managed_biguint!(USER_BALANCE)
            );
        })
        .assert_ok();

    // Merge user tokens
    let payments = vec![
        TxTokenTransfer {
            token_identifier: WRAPPED_FARM_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(USER_BALANCE),
        },
        TxTokenTransfer {
            token_identifier: WRAPPED_FARM_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(USER_BALANCE),
        },
    ];
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &payments, |sc| {
            sc.merge_wrapped_farm_tokens_endpoint(managed_address!(&farm_addr));
        })
        .assert_ok();

    // Total farm position should be correct
    setup
        .b_mock
        .execute_query(&setup.farm_locked_wrapper, |sc| {
            let first_user_total_farm_position = sc
                .user_total_farm_position(&managed_address!(&first_user))
                .get();
            assert_eq!(
                first_user_total_farm_position,
                managed_biguint!(USER_BALANCE * 2)
            );
        })
        .assert_ok();
}

#[test]
fn increase_proxy_farm_legacy_token_energy_negative_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let farm_addr = setup.farm_locked_wrapper.address_ref().clone();

    //////////////////////////////////////////// ENTER FARM /////////////////////////////////////

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LEGACY_LOCKED_TOKEN_ID,
            3,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    //////////////////////////////////////////// INCREASE ENERGY /////////////////////////////////////
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.increase_proxy_farm_token_energy_endpoint(LOCK_OPTIONS[1]);
            },
        )
        .assert_user_error("Invalid payments");
}

#[test]
fn total_farm_position_migration_mechanism_test() {
    let mut setup = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let first_user = setup.first_user.clone();
    let farm_addr = setup.farm_locked_wrapper.address_ref().clone();
    let user_balance = rust_biguint!(USER_BALANCE * 6);
    setup
        .b_mock
        .set_esdt_balance(&first_user, MEX_TOKEN_ID, &user_balance);

    // user locks tokens
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.simple_lock_wrapper,
            MEX_TOKEN_ID,
            0,
            &user_balance,
            |sc| {
                let user_payment = sc.lock_tokens_endpoint(LOCK_OPTIONS[1], OptionalValue::None);
                assert_eq!(user_payment.token_nonce, 2);
                assert_eq!(user_payment.amount, managed_biguint!(USER_BALANCE * 6));
            },
        )
        .assert_ok();

    // User enter farm 5 times before migration
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            2,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            2,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            2,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            2,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            LOCKED_TOKEN_ID,
            2,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // Simulate contract upgrade - total farm position is reset and migration nonce set
    setup
        .b_mock
        .execute_tx(
            &setup.owner,
            &setup.farm_locked_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.user_total_farm_position(&managed_address!(&first_user))
                    .set(managed_biguint!(0u64));
                sc.farm_position_migration_nonce().set(6u64);
            },
        )
        .assert_ok();

    setup
        .b_mock
        .execute_query(&setup.farm_locked_wrapper, |sc| {
            let first_user_total_farm_position = sc
                .user_total_farm_position(&managed_address!(&first_user))
                .get();
            assert_eq!(first_user_total_farm_position, managed_biguint!(0));
        })
        .assert_ok();

    // User enters farm again after migration
    let enter_payments = vec![
        TxTokenTransfer {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(USER_BALANCE),
        },
        TxTokenTransfer {
            token_identifier: WRAPPED_FARM_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(USER_BALANCE / 2),
        },
    ];
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &enter_payments, |sc| {
            sc.enter_farm_proxy_endpoint(managed_address!(&farm_addr), OptionalValue::None);
        })
        .assert_ok();

    // Check total farm position
    let mut user_total_farm_position = USER_BALANCE + (USER_BALANCE / 2);
    setup
        .b_mock
        .execute_query(&setup.farm_locked_wrapper, |sc| {
            let first_user_total_farm_position = sc
                .user_total_farm_position(&managed_address!(&first_user))
                .get();
            assert_eq!(
                first_user_total_farm_position,
                managed_biguint!(user_total_farm_position)
            );
        })
        .assert_ok();

    // Claim rewards with half old position
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            2,
            &rust_biguint!(USER_BALANCE / 2),
            |sc| {
                sc.claim_rewards_proxy(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // Check total farm position
    user_total_farm_position += USER_BALANCE / 2;
    setup
        .b_mock
        .execute_query(&setup.farm_locked_wrapper, |sc| {
            let first_user_total_farm_position = sc
                .user_total_farm_position(&managed_address!(&first_user))
                .get();
            assert_eq!(
                first_user_total_farm_position,
                managed_biguint!(user_total_farm_position)
            );
        })
        .assert_ok();

    // Exit farm with half old position
    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.proxy_wrapper,
            WRAPPED_FARM_TOKEN_ID,
            3,
            &rust_biguint!(USER_BALANCE / 2),
            |sc| {
                sc.exit_farm_proxy(managed_address!(&farm_addr), OptionalValue::None);
            },
        )
        .assert_ok();

    // Total farm position stays the same
    setup
        .b_mock
        .execute_query(&setup.farm_locked_wrapper, |sc| {
            let first_user_total_farm_position = sc
                .user_total_farm_position(&managed_address!(&first_user))
                .get();
            assert_eq!(
                first_user_total_farm_position,
                managed_biguint!(user_total_farm_position)
            );
        })
        .assert_ok();

    // Merge 2 partial old farm positions
    let merge_payments = vec![
        TxTokenTransfer {
            token_identifier: WRAPPED_FARM_TOKEN_ID.to_vec(),
            nonce: 4,
            value: rust_biguint!(USER_BALANCE / 2),
        },
        TxTokenTransfer {
            token_identifier: WRAPPED_FARM_TOKEN_ID.to_vec(),
            nonce: 5,
            value: rust_biguint!(USER_BALANCE / 4 * 3),
        },
    ];
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.proxy_wrapper, &merge_payments, |sc| {
            sc.merge_wrapped_farm_tokens_endpoint(managed_address!(&farm_addr));
        })
        .assert_ok();

    // Check final total farm position
    user_total_farm_position += USER_BALANCE / 2;
    user_total_farm_position += USER_BALANCE / 4 * 3;
    setup
        .b_mock
        .execute_query(&setup.farm_locked_wrapper, |sc| {
            let first_user_total_farm_position = sc
                .user_total_farm_position(&managed_address!(&first_user))
                .get();
            assert_eq!(
                first_user_total_farm_position,
                managed_biguint!(user_total_farm_position)
            );
        })
        .assert_ok();
}
