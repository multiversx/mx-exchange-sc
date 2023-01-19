use common_structs::{LockedAssetTokenAttributesEx, UnlockMilestoneEx, UnlockScheduleEx};
use energy_factory_mock::EnergyFactoryMock;
use energy_query::Energy;
use legacy_token_decode_module::LOCKED_TOKEN_ACTIVATION_NONCE;
use locked_token_wrapper::{
    wrapped_token::{WrappedTokenAttributes, WrappedTokenModule},
    LockedTokenWrapper,
};
use multiversx_sc::{
    storage::mappers::StorageTokenWrapper,
    types::{EsdtLocalRole, ManagedVec},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    whitebox::BlockchainStateWrapper, DebugApi,
};
use simple_lock::locked_token::LockedTokenAttributes;

static BASE_ASSET_TOKEN_ID: &[u8] = b"FREEEE-123456";
static LOCKED_TOKEN_ID: &[u8] = b"LOCKED-123456";
static LEGACY_LOCKED_TOKEN_ID: &[u8] = b"LEGACY-123456";
static WRAPPED_TOKEN_ID: &[u8] = b"WRAPPED-123456";

#[test]
fn token_wrap_unwrap_test() {
    let _ = DebugApi::dummy();
    let rust_zero = rust_biguint!(0);

    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_zero);
    let first_user = b_mock.create_user_account(&rust_zero);
    let second_user = b_mock.create_user_account(&rust_zero);
    let energy_factory = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        energy_factory_mock::contract_obj,
        "energy factory mock",
    );
    let locked_token_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        locked_token_wrapper::contract_obj,
        "locked token wrapper",
    );

    // setup wrapping SC
    b_mock
        .execute_tx(&owner, &locked_token_wrapper, &rust_zero, |sc| {
            sc.init(
                managed_token_id!(LEGACY_LOCKED_TOKEN_ID),
                managed_token_id!(LOCKED_TOKEN_ID),
                managed_address!(energy_factory.address_ref()),
            );

            sc.wrapped_token()
                .set_token_id(managed_token_id!(WRAPPED_TOKEN_ID));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        locked_token_wrapper.address_ref(),
        WRAPPED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    // simulate first user lock - 1_000 tokens for 20 epochs
    b_mock.set_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(1_000),
        &LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 20,
        },
    );

    b_mock
        .execute_tx(&owner, &energy_factory, &rust_zero, |sc| {
            let energy = Energy::new(
                (managed_biguint!(1_000) * 20u64).into(),
                0,
                managed_biguint!(1_000),
            );
            sc.user_energy(&managed_address!(&first_user)).set(&energy);
        })
        .assert_ok();

    // wrap 500 tokens
    b_mock
        .execute_esdt_transfer(
            &first_user,
            &locked_token_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(500),
            |sc| {
                let _ = sc.wrap_locked_token_endpoint();
            },
        )
        .assert_ok();

    b_mock.check_nft_balance(
        &first_user,
        WRAPPED_TOKEN_ID,
        1,
        &rust_biguint!(500),
        Some(&WrappedTokenAttributes::<DebugApi> {
            locked_token_id: managed_token_id!(LOCKED_TOKEN_ID),
            locked_token_nonce: 1,
        }),
    );

    // check energy after wrap
    b_mock
        .execute_query(&energy_factory, |sc| {
            let expected_energy = Energy::new(
                (managed_biguint!(500) * 20u64).into(),
                0,
                managed_biguint!(500),
            );
            let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(actual_energy, expected_energy);
        })
        .assert_ok();

    // simulate first user transfering wrapped tokens to second user
    b_mock.set_nft_balance(
        &second_user,
        WRAPPED_TOKEN_ID,
        1,
        &rust_biguint!(500),
        &WrappedTokenAttributes::<DebugApi> {
            locked_token_id: managed_token_id!(LOCKED_TOKEN_ID),
            locked_token_nonce: 1,
        },
    );

    // 5 epochs pass
    b_mock.set_block_epoch(5);

    // second user unwrap
    b_mock
        .execute_esdt_transfer(
            &second_user,
            &locked_token_wrapper,
            WRAPPED_TOKEN_ID,
            1,
            &rust_biguint!(500),
            |sc| {
                let _ = sc.unwrap_locked_token_endpoint();
            },
        )
        .assert_ok();

    b_mock.check_nft_balance(
        &second_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(500),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: 20,
        }),
    );

    // check energy after unwrap
    b_mock
        .execute_query(&energy_factory, |sc| {
            let expected_energy = Energy::new(
                (managed_biguint!(500) * 15u64).into(),
                5,
                managed_biguint!(500),
            );
            let actual_energy = sc.user_energy(&managed_address!(&second_user)).get();
            assert_eq!(actual_energy, expected_energy);
        })
        .assert_ok();
}

#[test]
fn both_tokens_wrap_unwrap_test() {
    let _ = DebugApi::dummy();
    let rust_zero = rust_biguint!(0);

    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_zero);
    let first_user = b_mock.create_user_account(&rust_zero);
    let second_user = b_mock.create_user_account(&rust_zero);
    let user_balance = 1_000u64;
    let energy_factory = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        energy_factory_mock::contract_obj,
        "energy factory mock",
    );
    let locked_token_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        locked_token_wrapper::contract_obj,
        "locked token wrapper",
    );

    // setup wrapping SC
    b_mock
        .execute_tx(&owner, &locked_token_wrapper, &rust_zero, |sc| {
            sc.init(
                managed_token_id!(LEGACY_LOCKED_TOKEN_ID),
                managed_token_id!(LOCKED_TOKEN_ID),
                managed_address!(energy_factory.address_ref()),
            );

            sc.wrapped_token()
                .set_token_id(managed_token_id!(WRAPPED_TOKEN_ID));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        locked_token_wrapper.address_ref(),
        WRAPPED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    let first_user_unlock_epoch = 1_700;
    b_mock.set_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(1_000),
        &LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: first_user_unlock_epoch,
        },
    );

    let mut current_epoch = 1_441;
    b_mock.set_block_epoch(current_epoch);

    let first_unlock_epoch = 1_531;
    let second_unlock_epoch = 1_621;
    let third_unlock_epoch = 1_711;
    let forth_unlock_epoch = 1_801;
    let mut unlock_milestones = ManagedVec::<DebugApi, UnlockMilestoneEx>::new();
    unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 20_000,
        unlock_epoch: first_unlock_epoch,
    });
    unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 20_000,
        unlock_epoch: second_unlock_epoch,
    });
    unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 20_000,
        unlock_epoch: third_unlock_epoch,
    });
    unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 40_000,
        unlock_epoch: forth_unlock_epoch,
    });
    let old_token_attributes = LockedAssetTokenAttributesEx {
        is_merged: false,
        unlock_schedule: UnlockScheduleEx { unlock_milestones },
    };

    b_mock.set_nft_balance(
        &second_user,
        LEGACY_LOCKED_TOKEN_ID,
        LOCKED_TOKEN_ACTIVATION_NONCE + 1,
        &rust_biguint!(user_balance),
        &old_token_attributes,
    );

    let mut second_user_energy_amount = 0u64;
    second_user_energy_amount +=
        20_000 * user_balance * (first_unlock_epoch - current_epoch) / 100_000u64;
    second_user_energy_amount +=
        20_000 * user_balance * (second_unlock_epoch - current_epoch) / 100_000u64;
    second_user_energy_amount +=
        20_000 * user_balance * (third_unlock_epoch - current_epoch) / 100_000u64;
    second_user_energy_amount +=
        40_000 * user_balance * (forth_unlock_epoch - current_epoch) / 100_000u64;

    b_mock
        .execute_tx(&owner, &energy_factory, &rust_zero, |sc| {
            let first_user_energy = Energy::new(
                (managed_biguint!(user_balance) * first_user_unlock_epoch).into(),
                0,
                managed_biguint!(user_balance),
            );
            let second_user_energy = Energy::new(
                (managed_biguint!(second_user_energy_amount)).into(),
                current_epoch,
                managed_biguint!(user_balance),
            );
            sc.user_energy(&managed_address!(&first_user))
                .set(&first_user_energy);
            sc.user_energy(&managed_address!(&second_user))
                .set(&second_user_energy);
        })
        .assert_ok();

    // wrap 500 tokens
    let user_half_balance = user_balance / 2;
    b_mock
        .execute_esdt_transfer(
            &first_user,
            &locked_token_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(user_half_balance),
            |sc| {
                let _ = sc.wrap_locked_token_endpoint();
            },
        )
        .assert_ok();

    b_mock
        .execute_esdt_transfer(
            &second_user,
            &locked_token_wrapper,
            LEGACY_LOCKED_TOKEN_ID,
            LOCKED_TOKEN_ACTIVATION_NONCE + 1,
            &rust_biguint!(user_half_balance),
            |sc| {
                let _ = sc.wrap_locked_token_endpoint();
            },
        )
        .assert_ok();

    b_mock.check_nft_balance(
        &first_user,
        WRAPPED_TOKEN_ID,
        1,
        &rust_biguint!(user_half_balance),
        Some(&WrappedTokenAttributes::<DebugApi> {
            locked_token_id: managed_token_id!(LOCKED_TOKEN_ID),
            locked_token_nonce: 1,
        }),
    );

    b_mock.check_nft_balance(
        &second_user,
        WRAPPED_TOKEN_ID,
        2,
        &rust_biguint!(user_half_balance),
        Some(&WrappedTokenAttributes::<DebugApi> {
            locked_token_id: managed_token_id!(LEGACY_LOCKED_TOKEN_ID),
            locked_token_nonce: LOCKED_TOKEN_ACTIVATION_NONCE + 1,
        }),
    );

    // check energy after wrap
    b_mock
        .execute_query(&energy_factory, |sc| {
            let expected_energy = Energy::new(
                (managed_biguint!(user_half_balance) * (first_user_unlock_epoch - current_epoch))
                    .into(),
                current_epoch,
                managed_biguint!(user_half_balance),
            );
            let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(actual_energy, expected_energy);
        })
        .assert_ok();

    b_mock
        .execute_query(&energy_factory, |sc| {
            let expected_energy = Energy::new(
                (managed_biguint!(second_user_energy_amount / 2)).into(),
                current_epoch,
                managed_biguint!(user_half_balance),
            );
            let actual_energy = sc.user_energy(&managed_address!(&second_user)).get();
            assert_eq!(actual_energy, expected_energy);
        })
        .assert_ok();

    // simulate the passing of 1 epoch
    current_epoch += 1;
    b_mock.set_block_epoch(current_epoch);

    // unwrap tokens
    b_mock
        .execute_esdt_transfer(
            &first_user,
            &locked_token_wrapper,
            WRAPPED_TOKEN_ID,
            1,
            &rust_biguint!(user_half_balance),
            |sc| {
                let _ = sc.unwrap_locked_token_endpoint();
            },
        )
        .assert_ok();

    b_mock
        .execute_esdt_transfer(
            &second_user,
            &locked_token_wrapper,
            WRAPPED_TOKEN_ID,
            2,
            &rust_biguint!(user_half_balance),
            |sc| {
                let _ = sc.unwrap_locked_token_endpoint();
            },
        )
        .assert_ok();

    // check balances
    b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(user_balance),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: first_user_unlock_epoch,
        }),
    );

    b_mock.check_nft_balance(
        &second_user,
        LEGACY_LOCKED_TOKEN_ID,
        LOCKED_TOKEN_ACTIVATION_NONCE + 1,
        &rust_biguint!(user_balance),
        Some(&old_token_attributes),
    );

    // check energy after unwrap
    b_mock
        .execute_query(&energy_factory, |sc| {
            let expected_energy = Energy::new(
                (managed_biguint!(user_balance) * (first_user_unlock_epoch - current_epoch)).into(),
                current_epoch,
                managed_biguint!(user_balance),
            );
            let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(actual_energy, expected_energy);
        })
        .assert_ok();

    let mut final_second_user_energy_amount = 0u64;
    final_second_user_energy_amount +=
        20_000 * user_balance * (first_unlock_epoch - current_epoch) / 100_000u64;
    final_second_user_energy_amount +=
        20_000 * user_balance * (second_unlock_epoch - current_epoch) / 100_000u64;
    final_second_user_energy_amount +=
        20_000 * user_balance * (third_unlock_epoch - current_epoch) / 100_000u64;
    final_second_user_energy_amount +=
        40_000 * user_balance * (forth_unlock_epoch - current_epoch) / 100_000u64;

    b_mock
        .execute_query(&energy_factory, |sc| {
            let expected_energy = Energy::new(
                (managed_biguint!(final_second_user_energy_amount)).into(),
                current_epoch,
                managed_biguint!(user_balance),
            );
            let actual_energy = sc.user_energy(&managed_address!(&second_user)).get();
            assert_eq!(actual_energy, expected_energy);
        })
        .assert_ok();
}
