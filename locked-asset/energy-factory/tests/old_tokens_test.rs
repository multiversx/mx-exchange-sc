mod energy_factory_setup;

use common_structs::{LockedAssetTokenAttributesEx, UnlockMilestoneEx, UnlockScheduleEx};
use elrond_wasm::types::{BigInt, ManagedVec};
use energy_factory::{
    energy::{Energy, EnergyModule},
    migration::SimpleLockMigrationModule,
};
use energy_factory_setup::*;
use simple_lock::locked_token::LockedTokenAttributes;

use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id_wrapped, rust_biguint, DebugApi,
};

#[test]
fn extend_lock_period_old_token_test() {
    let _ = DebugApi::dummy();
    let rust_zero = rust_biguint!(0);
    let mut setup =
        SimpleLockEnergySetup::new(energy_factory::contract_obj, token_unstake::contract_obj);

    setup.b_mock.set_block_epoch(1);

    let first_unlock_epoch = to_start_of_month(EPOCHS_IN_YEAR);
    let second_unlock_epoch = to_start_of_month(EPOCHS_IN_YEAR * 2);
    let mut unlock_milestones = ManagedVec::<DebugApi, UnlockMilestoneEx>::new();
    unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 40_000,
        unlock_epoch: first_unlock_epoch,
    });
    unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 60_000,
        unlock_epoch: second_unlock_epoch,
    });
    let old_token_attributes = LockedAssetTokenAttributesEx {
        is_merged: false,
        unlock_schedule: UnlockScheduleEx { unlock_milestones },
    };

    let first_user = setup.first_user.clone();
    setup.b_mock.set_nft_balance(
        &first_user,
        LEGACY_LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        &old_token_attributes,
    );

    let mut user_energy_amount = managed_biguint!(0);
    user_energy_amount += managed_biguint!(40_000) * USER_BALANCE * first_unlock_epoch / 100_000u32;
    user_energy_amount +=
        managed_biguint!(40_000) * USER_BALANCE * second_unlock_epoch / 100_000u32;

    setup
        .b_mock
        .execute_tx(&setup.owner, &setup.sc_wrapper, &rust_zero, |sc| {
            sc.update_energy_for_old_tokens(
                managed_address!(&first_user),
                managed_biguint!(USER_BALANCE),
                user_energy_amount.clone(),
            );

            let expected_energy = Energy::new(
                BigInt::from(user_energy_amount.clone()),
                1,
                managed_biguint!(USER_BALANCE),
            );
            let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();

    // extend to 4 years
    setup
        .extend_locking_period(
            &first_user,
            LEGACY_LOCKED_TOKEN_ID,
            1,
            USER_BALANCE,
            EPOCHS_IN_YEAR * 4,
        )
        .assert_ok();

    let new_unlock_epoch = to_start_of_month(EPOCHS_IN_YEAR * 4);
    setup.b_mock.check_nft_balance(
        &first_user,
        LOCKED_TOKEN_ID,
        1,
        &rust_biguint!(USER_BALANCE),
        Some(&LockedTokenAttributes::<DebugApi> {
            original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
            original_token_nonce: 0,
            unlock_epoch: new_unlock_epoch,
        }),
    );

    let mut energy_increase =
        managed_biguint!(40_000) * USER_BALANCE * (new_unlock_epoch - first_unlock_epoch)
            / 100_000u32;
    energy_increase +=
        managed_biguint!(60_000) * USER_BALANCE * (new_unlock_epoch - second_unlock_epoch)
            / 100_000u32;
    user_energy_amount += energy_increase;

    let actual_energy_after = setup.get_user_energy(&first_user);
    assert_eq!(
        to_rust_biguint(user_energy_amount.clone()),
        actual_energy_after
    );
}
