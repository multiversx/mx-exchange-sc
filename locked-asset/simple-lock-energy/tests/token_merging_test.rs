mod simple_lock_energy_setup;

use elrond_wasm::elrond_codec::multi_types::OptionalValue;
use elrond_wasm_debug::tx_mock::TxInputESDT;
use simple_lock::locked_token::LockedTokenAttributes;
use simple_lock_energy::token_merging::TokenMergingModule;
use simple_lock_energy_setup::*;

use elrond_wasm_debug::{managed_token_id_wrapped, rust_biguint, DebugApi};

#[test]
fn token_merging_test() {
    let _ = DebugApi::dummy();
    let mut setup = SimpleLockEnergySetup::new(simple_lock_energy::contract_obj);
    let first_user = setup.first_user.clone();

    let first_token_amount = 400_000;
    let first_token_unlock_epoch = to_start_of_month(LOCK_OPTIONS[1]);
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            first_token_amount,
            LOCK_OPTIONS[1],
        )
        .assert_ok();

    let second_token_amount = 100_000;
    let second_token_unlock_epoch = to_start_of_month(LOCK_OPTIONS[2]);
    setup
        .lock(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            second_token_amount,
            LOCK_OPTIONS[2],
        )
        .assert_ok();

    let payments = [
        TxInputESDT {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 1,
            value: rust_biguint!(400_000),
        },
        TxInputESDT {
            token_identifier: LOCKED_TOKEN_ID.to_vec(),
            nonce: 2,
            value: rust_biguint!(100_000),
        },
    ];
    setup
        .b_mock
        .execute_esdt_multi_transfer(&first_user, &setup.sc_wrapper, &payments[..], |sc| {
            let _ = sc.merge_tokens_endpoint(OptionalValue::None);
        })
        .assert_ok();

    assert_eq!(first_token_unlock_epoch, 360);
    assert_eq!(second_token_unlock_epoch, 720);

    // (400_000 * 360 + 100_000 * 720) / 500_000 = 432
    // -> start of month (upper) = 450
    let expected_merged_token_unlock_epoch = 450;
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        3,
        &rust_biguint!(first_token_amount + second_token_amount),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: expected_merged_token_unlock_epoch,
        }),
    );

    let expected_energy = rust_biguint!(500_000) * expected_merged_token_unlock_epoch;
    let actual_energy = setup.get_user_energy(&first_user);
    assert_eq!(expected_energy, actual_energy);
}
