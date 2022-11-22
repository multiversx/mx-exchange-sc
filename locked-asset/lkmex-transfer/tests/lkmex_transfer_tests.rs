use elrond_wasm::elrond_codec::multi_types::OptionalValue;
use elrond_wasm::storage::mappers::StorageTokenWrapper;
use elrond_wasm::types::{BigInt, EsdtLocalRole, MultiValueEncoded};
use elrond_wasm_debug::{managed_address, managed_biguint, testing_framework::*};
use elrond_wasm_debug::{managed_token_id, rust_biguint};

use elrond_wasm_modules::pause::PauseModule;
use energy_factory::energy::EnergyModule;
use energy_factory::lock_options::LockOptionsModule;
use energy_factory::locked_token_transfer::LockedTokenTransferModule;
use energy_factory::SimpleLockEnergy;
use energy_query::Energy;
use lkmex_transfer::LkmexTransfer;
use simple_lock::locked_token::LockedTokenModule;

pub const EPOCHS_IN_YEAR: u64 = 360;
pub const EPOCHS_IN_WEEK: u64 = 7;
pub const USER_BALANCE: u64 = 1_000_000_000_000_000_000;

pub static BASE_ASSET_TOKEN_ID: &[u8] = b"MEX-123456";
pub static LOCKED_TOKEN_ID: &[u8] = b"LOCKED-123456";
pub static LEGACY_LOCKED_TOKEN_ID: &[u8] = b"LEGACY-123456";

pub static LOCK_OPTIONS: &[u64] = &[EPOCHS_IN_YEAR, 2 * EPOCHS_IN_YEAR, 4 * EPOCHS_IN_YEAR]; // 1, 2 or 4 years
pub static PENALTY_PERCENTAGES: &[u64] = &[4_000, 6_000, 8_000];

#[test]
fn transfer_locked_token_test() {
    let rust_zero = rust_biguint!(0);
    let mut b_mock = BlockchainStateWrapper::new();

    let user_addr = b_mock.create_user_account(&rust_zero);
    let claimer_addr = b_mock.create_user_account(&rust_zero);
    let owner_addr = b_mock.create_user_account(&rust_zero);
    let transfer_sc_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        lkmex_transfer::contract_obj,
        "Some path",
    );
    let factory_sc_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        energy_factory::contract_obj,
        "Some other path",
    );

    b_mock.set_block_epoch(5);

    // Setup transfer SC
    b_mock
        .execute_tx(&owner_addr, &transfer_sc_wrapper, &rust_zero, |sc| {
            sc.init(
                managed_address!(factory_sc_wrapper.address_ref()),
                managed_token_id!(LOCKED_TOKEN_ID),
                4,
                6,
            );
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        transfer_sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[EsdtLocalRole::Transfer],
    );

    // setup energy factory SC
    b_mock
        .execute_tx(&owner_addr, &factory_sc_wrapper, &rust_zero, |sc| {
            let mut lock_options = MultiValueEncoded::new();
            for (option, penalty) in LOCK_OPTIONS.iter().zip(PENALTY_PERCENTAGES.iter()) {
                lock_options.push((*option, *penalty).into());
            }

            // sc addresses don't matter here, we don't test that part
            sc.init(
                managed_token_id!(BASE_ASSET_TOKEN_ID),
                managed_token_id!(LEGACY_LOCKED_TOKEN_ID),
                managed_address!(transfer_sc_wrapper.address_ref()),
                0,
                lock_options,
            );

            sc.locked_token()
                .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            sc.token_transfer_whitelist()
                .add(&managed_address!(transfer_sc_wrapper.address_ref()));
            sc.set_paused(false);
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        factory_sc_wrapper.address_ref(),
        BASE_ASSET_TOKEN_ID,
        &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
    );
    b_mock.set_esdt_local_roles(
        factory_sc_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
            EsdtLocalRole::Transfer,
        ],
    );
    b_mock.set_esdt_local_roles(
        factory_sc_wrapper.address_ref(),
        LEGACY_LOCKED_TOKEN_ID,
        &[EsdtLocalRole::NftBurn],
    );

    // setup user balance

    b_mock.set_esdt_balance(
        &user_addr,
        BASE_ASSET_TOKEN_ID,
        &rust_biguint!(USER_BALANCE),
    );

    // lock tokens
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &factory_sc_wrapper,
            BASE_ASSET_TOKEN_ID,
            0,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.lock_tokens_endpoint(LOCK_OPTIONS[0], OptionalValue::None);

                let unlock_epoch = sc.unlock_epoch_to_start_of_month(5 + LOCK_OPTIONS[0]);
                let lock_epochs = unlock_epoch - 5;
                let expected_energy_amount =
                    BigInt::from(USER_BALANCE as i64) * BigInt::from(lock_epochs as i64);
                let expected_energy =
                    Energy::new(expected_energy_amount, 5, managed_biguint!(USER_BALANCE));
                let actual_energy = sc.user_energy(&managed_address!(&user_addr)).get();
                assert_eq!(expected_energy, actual_energy);
            },
        )
        .assert_ok();

    // transfer half of the LKMEX to other user
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &transfer_sc_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE / 2),
            |sc| {
                sc.lock_funds(managed_address!(&claimer_addr));
            },
        )
        .assert_ok();

    // check first user energy after transfer
    b_mock
        .execute_query(&factory_sc_wrapper, |sc| {
            let unlock_epoch = sc.unlock_epoch_to_start_of_month(5 + LOCK_OPTIONS[0]);
            let lock_epochs = unlock_epoch - 5;
            let expected_energy_amount =
                BigInt::from((USER_BALANCE / 2) as i64) * BigInt::from(lock_epochs as i64);
            let expected_energy = Energy::new(
                expected_energy_amount,
                5,
                managed_biguint!(USER_BALANCE / 2),
            );
            let actual_energy = sc.user_energy(&managed_address!(&user_addr)).get();
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();

    // pass 5 epochs
    b_mock.set_block_epoch(10);

    // second user claim
    b_mock
        .execute_tx(&claimer_addr, &transfer_sc_wrapper, &rust_zero, |sc| {
            sc.withdraw();
        })
        .assert_ok();

    // check second user energy
    b_mock
        .execute_query(&factory_sc_wrapper, |sc| {
            let unlock_epoch = sc.unlock_epoch_to_start_of_month(5 + LOCK_OPTIONS[0]);
            let lock_epochs = unlock_epoch - 5;
            let expected_energy_amount =
                BigInt::from((USER_BALANCE / 2) as i64) * BigInt::from(lock_epochs as i64);
            let mut expected_energy = Energy::new(
                expected_energy_amount,
                5,
                managed_biguint!(USER_BALANCE / 2),
            );
            expected_energy.deplete(10);

            let actual_energy = sc.user_energy(&managed_address!(&claimer_addr)).get();
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();
}
