#![allow(deprecated)]

use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc::storage::mappers::StorageTokenWrapper;
use multiversx_sc::types::{BigInt, EsdtLocalRole, MultiValueEncoded};
use multiversx_sc_scenario::{managed_address, managed_biguint, whitebox_legacy::*};
use multiversx_sc_scenario::{managed_token_id, rust_biguint};

use energy_factory::energy::EnergyModule;
use energy_factory::lock_options::LockOptionsModule;
use energy_factory::locked_token_transfer::LockedTokenTransferModule;
use energy_factory::SimpleLockEnergy;
use energy_query::Energy;
use lkmex_transfer::LkmexTransfer;
use multiversx_sc_modules::pause::PauseModule;
use permissions_module::PermissionsModule;
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
            sc.withdraw(managed_address!(&user_addr));
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

#[test]
fn transfer_cooldowns_test() {
    let rust_zero = rust_biguint!(0);
    let mut b_mock = BlockchainStateWrapper::new();

    let user_addr = b_mock.create_user_account(&rust_zero);
    let user_addr2 = b_mock.create_user_account(&rust_zero);
    let claimer_addr = b_mock.create_user_account(&rust_zero);
    let claimer_addr2 = b_mock.create_user_account(&rust_zero);
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

    let mut blockchain_epoch = 5u64;
    b_mock.set_block_epoch(blockchain_epoch);

    // Setup transfer SC
    b_mock
        .execute_tx(&owner_addr, &transfer_sc_wrapper, &rust_zero, |sc| {
            sc.init(
                managed_address!(factory_sc_wrapper.address_ref()),
                managed_token_id!(LOCKED_TOKEN_ID),
                30,
                30,
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

    b_mock.set_esdt_balance(
        &user_addr2,
        BASE_ASSET_TOKEN_ID,
        &rust_biguint!(USER_BALANCE),
    );

    // user 1 lock tokens
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

    // user 2 lock tokens
    b_mock
        .execute_esdt_transfer(
            &user_addr2,
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
                let actual_energy = sc.user_energy(&managed_address!(&user_addr2)).get();
                assert_eq!(expected_energy, actual_energy);
            },
        )
        .assert_ok();

    // user 1 transfer half of the XMEX to the receiver
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

    blockchain_epoch += 5u64;
    b_mock.set_block_epoch(blockchain_epoch);

    // user 2 transfer half of the XMEX to the receiver
    // transfer happens after 5 epochs after the first user transfer
    b_mock
        .execute_esdt_transfer(
            &user_addr2,
            &transfer_sc_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE / 2),
            |sc| {
                sc.lock_funds(managed_address!(&claimer_addr));
            },
        )
        .assert_ok();

    // transfer user 1 rest of the XMEX to receiver 2
    // error sender still on cooldown
    b_mock
        .execute_esdt_transfer(
            &user_addr,
            &transfer_sc_wrapper,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE / 2),
            |sc| {
                sc.lock_funds(managed_address!(&claimer_addr2));
            },
        )
        .assert_error(4, "caller cannot use this contract at this time");

    // second user claim
    // error claimed too soon
    b_mock
        .execute_tx(&claimer_addr, &transfer_sc_wrapper, &rust_zero, |sc| {
            sc.withdraw(managed_address!(&user_addr));
        })
        .assert_error(4, "requested funds are still locked");

    blockchain_epoch += 30u64;
    b_mock.set_block_epoch(blockchain_epoch);

    // receiver claims tokens from user 1 after 30 epochs
    b_mock
        .execute_tx(&claimer_addr, &transfer_sc_wrapper, &rust_zero, |sc| {
            sc.withdraw(managed_address!(&user_addr));
        })
        .assert_ok();

    // receiver claims tokens from user 2 after 30 epochs
    // cooldown error for claimer, the cooldown was reset after user 1 transfer was withdrawn
    b_mock
        .execute_tx(&claimer_addr, &transfer_sc_wrapper, &rust_zero, |sc| {
            sc.withdraw(managed_address!(&user_addr2));
        })
        .assert_error(4, "caller cannot use this contract at this time");

    blockchain_epoch += 31u64;
    b_mock.set_block_epoch(blockchain_epoch);

    // receiver claims tokens from user 1 after 31 more epochs - starting from user 1 transfer withdrawal
    b_mock
        .execute_tx(&claimer_addr, &transfer_sc_wrapper, &rust_zero, |sc| {
            sc.withdraw(managed_address!(&user_addr2));
        })
        .assert_ok();
}

#[test]
fn cancel_transfer_test() {
    let rust_zero = rust_biguint!(0);
    let mut b_mock = BlockchainStateWrapper::new();

    let user_addr = b_mock.create_user_account(&rust_zero);
    let claimer_addr = b_mock.create_user_account(&rust_zero);
    let owner_addr = b_mock.create_user_account(&rust_zero);
    let admin_addr = b_mock.create_user_account(&rust_zero);
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

    // add admin
    b_mock
        .execute_tx(&owner_addr, &transfer_sc_wrapper, &rust_zero, |sc| {
            sc.add_admin_endpoint(managed_address!(&admin_addr));
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

    // cancel transfer by the admin
    b_mock
        .execute_tx(&admin_addr, &transfer_sc_wrapper, &rust_zero, |sc| {
            sc.cancel_transfer(
                managed_address!(&user_addr),
                managed_address!(&claimer_addr),
            );
        })
        .assert_ok();

    // check first user energy after cancel transfer
    b_mock
        .execute_query(&factory_sc_wrapper, |sc| {
            let unlock_epoch = sc.unlock_epoch_to_start_of_month(5 + LOCK_OPTIONS[0]);
            let lock_epochs = unlock_epoch - 5;
            let expected_energy_amount =
                BigInt::from((USER_BALANCE) as i64) * BigInt::from(lock_epochs as i64);
            let expected_energy =
                Energy::new(expected_energy_amount, 5, managed_biguint!(USER_BALANCE));
            let actual_energy = sc.user_energy(&managed_address!(&user_addr)).get();
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();
}
