#![allow(deprecated)]

mod simple_lock_test_setup;

use common_structs::FarmTokenAttributes;
use config::ConfigModule;
use energy_factory::energy::EnergyModule;
use energy_query::Energy;

use multiversx_sc::{
    codec::{multi_types::OptionalValue, Empty},
    types::{BigInt, EsdtLocalRole, EsdtTokenPayment},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    whitebox_legacy::TxTokenTransfer, DebugApi,
};
use num_traits::ToPrimitive;
use proxy_dex::{
    proxy_farm::ProxyFarmModule, proxy_pair::ProxyPairModule,
    wrapped_farm_attributes::WrappedFarmTokenAttributes,
    wrapped_lp_attributes::WrappedLpTokenAttributes,
};

#[test]
fn destroy_farm_locked_tokens_test() {
    let mut setup = SimpleLockSetup::new(
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
            let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
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

    current_epoch += 5; // applies penalty on exit
    setup.b_mock.set_block_epoch(current_epoch);
    setup.b_mock.set_block_nonce(100);

    ////////////////////////////////////////////// DESTROY FARM /////////////////////////////////////
    
    // should be 500_000_000, but ends up so due to approximations
    let removed_locked_token_amount = rust_biguint!(499_999_000);
    // should be 250_000_000, but ends up so due to approximations
    let removed_other_token_amount = rust_biguint!(249_999_500);
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
                let output_payments = sc.destroy_farm_proxy(
                    managed_address!(&farm_locked_addr),
                    managed_address!(&pair_addr),
                    managed_biguint!(1),
                    managed_biguint!(1),
                    OptionalValue::None,
                );

                let output_vec = output_payments.to_vec();

                assert_eq!(output_payments.len(), 3);
                assert_eq!(
                    output_vec.get(0).amount.to_u64().unwrap(),
                    removed_locked_token_amount.to_u64().unwrap()
                );
                assert_eq!(
                    output_vec.get(1).amount.to_u64().unwrap(),
                    removed_other_token_amount.to_u64().unwrap()
                );
                assert_eq!(output_vec.get(2).amount.to_u64().unwrap(), 0u64);
            },
        )
        .assert_ok();

    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        WRAPPED_FARM_TOKEN_ID,
        1,
        &(&expected_lp_token_amount / 2u64),
        None,
    );

    setup.b_mock.check_nft_balance::<Empty>(
        &first_user,
        WRAPPED_LP_TOKEN_ID,
        1,
        &rust_biguint!(0u64),
        None,
    );
}
