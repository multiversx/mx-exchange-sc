use common_structs::{
    LockedAssetTokenAttributesEx, UnlockMilestone, UnlockMilestoneEx, UnlockScheduleEx,
};
use elrond_wasm::{
    storage::mappers::StorageTokenWrapper,
    types::{EsdtLocalRole, EsdtTokenPayment, ManagedVec, MultiValueEncoded},
};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    testing_framework::BlockchainStateWrapper, DebugApi,
};
use elrond_wasm_modules::pause::PauseModule;
use factory::{
    locked_asset::LockedAssetModule, migration::LockedTokenMigrationModule, LockedAssetFactory,
};
use simple_lock::locked_token::{LockedTokenAttributes, LockedTokenModule};
use simple_lock_energy::{migration::SimpleLockMigrationModule, SimpleLockEnergy};

static MEX_TOKEN_ID: &[u8] = b"MEX-123456";
static OLD_LKMEX_TOKEN_ID: &[u8] = b"LKMEX-123456";
static NEW_LKMEX_TOKEN_ID: &[u8] = b"LKMEX-abcdef";

const USER_BALANCE: u64 = 1_000_000_000_000_000_000;
const EPOCHS_IN_YEAR: u64 = 365;

#[test]
fn old_to_new_locked_token_migration_test() {
    let _ = DebugApi::dummy();
    let mut b_mock = BlockchainStateWrapper::new();
    let rust_zero = rust_biguint!(0);
    let owner = b_mock.create_user_account(&rust_zero);
    let user = b_mock.create_user_account(&rust_zero);
    let old_factory = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        factory::contract_obj,
        "old factory",
    );
    let new_factory = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        simple_lock_energy::contract_obj,
        "new factory",
    );
    let fees_collector = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        fees_collector::contract_obj,
        "fees collector",
    );

    b_mock.set_esdt_balance(&user, MEX_TOKEN_ID, &rust_biguint!(USER_BALANCE));

    // setup old factory
    b_mock
        .execute_tx(&owner, &old_factory, &rust_zero, |sc| {
            let mut unlock_milestones = MultiValueEncoded::new();
            unlock_milestones.push(UnlockMilestone {
                unlock_percent: 40,
                unlock_epoch: EPOCHS_IN_YEAR,
            });
            unlock_milestones.push(UnlockMilestone {
                unlock_percent: 60,
                unlock_epoch: 3 * EPOCHS_IN_YEAR,
            });

            sc.init(managed_token_id!(MEX_TOKEN_ID), unlock_milestones);
            sc.set_paused(false);
            sc.locked_asset_token()
                .set_token_id(&managed_token_id!(OLD_LKMEX_TOKEN_ID));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        old_factory.address_ref(),
        MEX_TOKEN_ID,
        &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
    );
    b_mock.set_esdt_local_roles(
        old_factory.address_ref(),
        OLD_LKMEX_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    // user lock in old factory
    b_mock
        .execute_esdt_transfer(
            &user,
            &old_factory,
            MEX_TOKEN_ID,
            0,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                let lock_result = sc.lock_assets();
                let expected_result = EsdtTokenPayment::new(
                    managed_token_id!(OLD_LKMEX_TOKEN_ID),
                    1,
                    managed_biguint!(USER_BALANCE),
                );
                assert_eq!(lock_result, expected_result);
            },
        )
        .assert_ok();

    let mut expected_unlock_milestones = ManagedVec::<DebugApi, UnlockMilestoneEx>::new();
    expected_unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 40000,
        unlock_epoch: EPOCHS_IN_YEAR,
    });
    expected_unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 60000,
        unlock_epoch: EPOCHS_IN_YEAR * 3,
    });
    b_mock.check_nft_balance(
        &user,
        OLD_LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&LockedAssetTokenAttributesEx {
            is_merged: false,
            unlock_schedule: UnlockScheduleEx {
                unlock_milestones: expected_unlock_milestones,
            },
        }),
    );

    // setup new factory
    b_mock
        .execute_tx(&owner, &new_factory, &rust_zero, |sc| {
            let mut lock_options = MultiValueEncoded::new();
            lock_options.push(EPOCHS_IN_YEAR);
            lock_options.push(3 * EPOCHS_IN_YEAR);
            lock_options.push(5 * EPOCHS_IN_YEAR);

            sc.init(
                managed_token_id!(MEX_TOKEN_ID),
                5_000,
                5_000,
                5_000,
                managed_address!(fees_collector.address_ref()),
                lock_options,
            );
            sc.locked_token()
                .set_token_id(&managed_token_id!(NEW_LKMEX_TOKEN_ID));
            sc.set_paused(false);
            sc.set_old_locked_asset_factory_address(managed_address!(old_factory.address_ref()));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        new_factory.address_ref(),
        NEW_LKMEX_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    // start migration
    b_mock
        .execute_tx(&owner, &old_factory, &rust_zero, |sc| {
            sc.start_migration(managed_address!(new_factory.address_ref()));
        })
        .assert_ok();

    // migrate old to new token
    b_mock
        .execute_esdt_transfer(
            &user,
            &old_factory,
            OLD_LKMEX_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.migrate_to_new_factory();
            },
        )
        .assert_ok();

    let expected_new_token_attributes = LockedTokenAttributes::<DebugApi> {
        original_token_id: managed_token_id_wrapped!(MEX_TOKEN_ID),
        original_token_nonce: 0,
        unlock_epoch: 780, // (40 * 365 + 60 * 3 * 365) / 100 ~= 803 -> start of month = 780
    };
    b_mock.check_nft_balance(
        &user,
        NEW_LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&expected_new_token_attributes),
    );
}
