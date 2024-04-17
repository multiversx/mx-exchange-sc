#![allow(deprecated)]

mod energy_factory_setup;

use energy_factory::virtual_lock::VirtualLockModule;
use energy_factory_setup::*;
use sc_whitelist_module::SCWhitelistModule;
use simple_lock::locked_token::LockedTokenAttributes;

use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    DebugApi,
};

#[test]
fn virtual_lock_test() {
    let mut setup = SimpleLockEnergySetup::new(energy_factory::contract_obj);
    let first_user = setup.first_user.clone();
    let second_user = setup.second_user.clone();

    // not whitelisted
    setup
        .b_mock
        .execute_tx(&first_user, &setup.sc_wrapper, &rust_biguint!(0), |sc| {
            sc.lock_virtual(
                managed_token_id!(BASE_ASSET_TOKEN_ID),
                managed_biguint!(1_000),
                LOCK_OPTIONS[0],
                managed_address!(&second_user),
                managed_address!(&second_user),
            );
        })
        .assert_user_error("Item not whitelisted");

    // wrong token
    setup
        .b_mock
        .execute_tx(&first_user, &setup.sc_wrapper, &rust_biguint!(0), |sc| {
            sc.lock_virtual(
                managed_token_id!(b"RANDTOK-123456"),
                managed_biguint!(1_000),
                LOCK_OPTIONS[0],
                managed_address!(&second_user),
                managed_address!(&second_user),
            );
        })
        .assert_user_error("May only lock the base asset token");

    // lock virtual ok
    setup
        .b_mock
        .execute_tx(&first_user, &setup.sc_wrapper, &rust_biguint!(0), |sc| {
            sc.sc_whitelist_addresses()
                .add(&managed_address!(&first_user));

            sc.lock_virtual(
                managed_token_id!(BASE_ASSET_TOKEN_ID),
                managed_biguint!(1_000),
                LOCK_OPTIONS[0],
                managed_address!(&second_user),
                managed_address!(&second_user),
            );
        })
        .assert_ok();

    setup.b_mock.check_nft_balance(
        &second_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(1_000),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 360,
        }),
    );
}
