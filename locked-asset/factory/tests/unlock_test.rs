#![allow(deprecated)]

use common_structs::{LockedAssetTokenAttributesEx, UnlockMilestoneEx, UnlockScheduleEx};
use multiversx_sc::types::{BigInt, MultiValueEncoded};
use multiversx_sc::types::{EsdtTokenPayment, ManagedVec, TokenIdentifier};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, rust_biguint, whitebox_legacy::*, DebugApi,
};

const SC_WASM_PATH: &str = "output/factory.wasm";

use energy_factory::energy::{Energy, EnergyModule};
use energy_factory::migration::SimpleLockMigrationModule;
use factory::locked_asset_token_merge::*;
use factory::{locked_asset::*, LockedAssetFactory};
use factory_setup::*;
use multiversx_sc_modules::pause::PauseModule;

mod factory_setup;

#[test]
fn test_unlock_100mil_1mil() {
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let factory = blockchain_wrapper.create_sc_account(
        &rust_biguint!(0),
        None,
        factory::contract_obj,
        SC_WASM_PATH,
    );

    blockchain_wrapper
        .execute_query(&factory, |sc| {
            let mut tokens = ManagedVec::new();
            let token1 = LockedTokenEx::<DebugApi> {
                token_amount: EsdtTokenPayment {
                    token_identifier: TokenIdentifier::from_esdt_bytes(&[]), //placeholder
                    token_nonce: 0,                                          //placeholder
                    amount: managed_biguint!(100_000_000),
                },
                attributes: LockedAssetTokenAttributesEx {
                    unlock_schedule: UnlockScheduleEx {
                        unlock_milestones: ManagedVec::from(vec![
                            UnlockMilestoneEx {
                                unlock_epoch: 0,
                                unlock_percent: 10_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 360,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 390,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 420,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 450,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 480,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 510,
                                unlock_percent: 15_000,
                            },
                        ]),
                    },
                    is_merged: false,
                },
            };
            let token2 = LockedTokenEx::<DebugApi> {
                token_amount: EsdtTokenPayment {
                    token_identifier: TokenIdentifier::from_esdt_bytes(&[]), //placeholder
                    token_nonce: 0,                                          //placeholder
                    amount: managed_biguint!(1_000_000),
                },
                attributes: LockedAssetTokenAttributesEx {
                    unlock_schedule: UnlockScheduleEx {
                        unlock_milestones: ManagedVec::from(vec![
                            UnlockMilestoneEx {
                                unlock_epoch: 360,
                                unlock_percent: 16_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 390,
                                unlock_percent: 16_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 420,
                                unlock_percent: 17_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 450,
                                unlock_percent: 17_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 480,
                                unlock_percent: 17_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 510,
                                unlock_percent: 17_000,
                            },
                        ]),
                    },
                    is_merged: false,
                },
            };

            tokens.push(token1);
            tokens.push(token2);

            let result = sc.aggregated_unlock_schedule(&tokens);
            let result = result.unlock_milestones;

            let el = result.get(0);
            assert_eq!(el.unlock_epoch, 0);
            assert_eq!(el.unlock_percent, 9_901);

            let el = result.get(1);
            assert_eq!(el.unlock_epoch, 360);
            assert_eq!(el.unlock_percent, 15_010);

            let el = result.get(2);
            assert_eq!(el.unlock_epoch, 390);
            assert_eq!(el.unlock_percent, 15_010);

            let el = result.get(3);
            assert_eq!(el.unlock_epoch, 420);
            assert_eq!(el.unlock_percent, 15_019);

            let el = result.get(4);
            assert_eq!(el.unlock_epoch, 450);
            assert_eq!(el.unlock_percent, 15_020);

            let el = result.get(5);
            assert_eq!(el.unlock_epoch, 480);
            assert_eq!(el.unlock_percent, 15_020);

            let el = result.get(6);
            assert_eq!(el.unlock_epoch, 510);
            assert_eq!(el.unlock_percent, 15_020);
        })
        .assert_ok();
}

#[test]
fn test_unlock_1mil_100mil() {
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let factory = blockchain_wrapper.create_sc_account(
        &rust_biguint!(0),
        None,
        factory::contract_obj,
        SC_WASM_PATH,
    );

    blockchain_wrapper
        .execute_query(&factory, |sc| {
            let mut tokens = ManagedVec::new();
            let token1 = LockedTokenEx::<DebugApi> {
                token_amount: EsdtTokenPayment {
                    token_identifier: TokenIdentifier::from_esdt_bytes(&[]), //placeholder
                    token_nonce: 0,                                          //placeholder
                    amount: managed_biguint!(1_000_000),
                },
                attributes: LockedAssetTokenAttributesEx {
                    unlock_schedule: UnlockScheduleEx {
                        unlock_milestones: ManagedVec::from(vec![
                            UnlockMilestoneEx {
                                unlock_epoch: 0,
                                unlock_percent: 10_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 360,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 390,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 420,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 450,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 480,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 510,
                                unlock_percent: 15_000,
                            },
                        ]),
                    },
                    is_merged: false,
                },
            };
            let token2 = LockedTokenEx::<DebugApi> {
                token_amount: EsdtTokenPayment {
                    token_identifier: TokenIdentifier::from_esdt_bytes(&[]), //placeholder
                    token_nonce: 0,                                          //placeholder
                    amount: managed_biguint!(100_000_000),
                },
                attributes: LockedAssetTokenAttributesEx {
                    unlock_schedule: UnlockScheduleEx {
                        unlock_milestones: ManagedVec::from(vec![
                            UnlockMilestoneEx {
                                unlock_epoch: 360,
                                unlock_percent: 16_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 390,
                                unlock_percent: 16_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 420,
                                unlock_percent: 17_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 450,
                                unlock_percent: 17_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 480,
                                unlock_percent: 17_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 510,
                                unlock_percent: 17_000,
                            },
                        ]),
                    },
                    is_merged: false,
                },
            };

            tokens.push(token1);
            tokens.push(token2);

            let result = sc.aggregated_unlock_schedule(&tokens);
            let result = result.unlock_milestones;

            let el = result.get(0);
            assert_eq!(el.unlock_epoch, 0);
            assert_eq!(el.unlock_percent, 99);

            let el = result.get(1);
            assert_eq!(el.unlock_epoch, 360);
            assert_eq!(el.unlock_percent, 15_990);

            let el = result.get(2);
            assert_eq!(el.unlock_epoch, 390);
            assert_eq!(el.unlock_percent, 15_990);

            let el = result.get(3);
            assert_eq!(el.unlock_epoch, 420);
            assert_eq!(el.unlock_percent, 16_980);

            let el = result.get(4);
            assert_eq!(el.unlock_epoch, 450);
            assert_eq!(el.unlock_percent, 16_980);

            let el = result.get(5);
            assert_eq!(el.unlock_epoch, 480);
            assert_eq!(el.unlock_percent, 16_980);

            let el = result.get(6);
            assert_eq!(el.unlock_epoch, 510);
            assert_eq!(el.unlock_percent, 16981);
        })
        .assert_ok();
}

#[test]
fn test_unlock_60_40() {
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let factory = blockchain_wrapper.create_sc_account(
        &rust_biguint!(0),
        None,
        factory::contract_obj,
        SC_WASM_PATH,
    );

    blockchain_wrapper
        .execute_query(&factory, |sc| {
            let mut tokens = ManagedVec::new();
            let token1 = LockedTokenEx::<DebugApi> {
                token_amount: EsdtTokenPayment {
                    token_identifier: TokenIdentifier::from_esdt_bytes(&[]), //placeholder
                    token_nonce: 0,                                          //placeholder
                    amount: managed_biguint!(60_000),
                },
                attributes: LockedAssetTokenAttributesEx {
                    unlock_schedule: UnlockScheduleEx {
                        unlock_milestones: ManagedVec::from(vec![
                            UnlockMilestoneEx {
                                unlock_epoch: 0,
                                unlock_percent: 10_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 360,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 390,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 420,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 450,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 480,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 510,
                                unlock_percent: 15_000,
                            },
                        ]),
                    },
                    is_merged: false,
                },
            };
            let token2 = LockedTokenEx::<DebugApi> {
                token_amount: EsdtTokenPayment {
                    token_identifier: TokenIdentifier::from_esdt_bytes(&[]), //placeholder
                    token_nonce: 0,                                          //placeholder
                    amount: managed_biguint!(40_000),
                },
                attributes: LockedAssetTokenAttributesEx {
                    unlock_schedule: UnlockScheduleEx {
                        unlock_milestones: ManagedVec::from(vec![
                            UnlockMilestoneEx {
                                unlock_epoch: 360,
                                unlock_percent: 16_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 390,
                                unlock_percent: 16_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 420,
                                unlock_percent: 17_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 450,
                                unlock_percent: 17_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 480,
                                unlock_percent: 17_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 510,
                                unlock_percent: 17_000,
                            },
                        ]),
                    },
                    is_merged: false,
                },
            };

            tokens.push(token1);
            tokens.push(token2);

            let result = sc.aggregated_unlock_schedule(&tokens);
            let result = result.unlock_milestones;

            let el = result.get(0);
            assert_eq!(el.unlock_epoch, 0);
            assert_eq!(el.unlock_percent, 6_000);

            let el = result.get(1);
            assert_eq!(el.unlock_epoch, 360);
            assert_eq!(el.unlock_percent, 15_400);

            let el = result.get(2);
            assert_eq!(el.unlock_epoch, 390);
            assert_eq!(el.unlock_percent, 15_400);

            let el = result.get(3);
            assert_eq!(el.unlock_epoch, 420);
            assert_eq!(el.unlock_percent, 15_800);

            let el = result.get(4);
            assert_eq!(el.unlock_epoch, 450);
            assert_eq!(el.unlock_percent, 15_800);

            let el = result.get(5);
            assert_eq!(el.unlock_epoch, 480);
            assert_eq!(el.unlock_percent, 15_800);

            let el = result.get(6);
            assert_eq!(el.unlock_epoch, 510);
            assert_eq!(el.unlock_percent, 15_800);
        })
        .assert_ok();
}

#[test]
fn test_unlock_40_60() {
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let factory = blockchain_wrapper.create_sc_account(
        &rust_biguint!(0),
        None,
        factory::contract_obj,
        SC_WASM_PATH,
    );

    blockchain_wrapper
        .execute_query(&factory, |sc| {
            let mut tokens = ManagedVec::new();
            let token1 = LockedTokenEx::<DebugApi> {
                token_amount: EsdtTokenPayment {
                    token_identifier: TokenIdentifier::from_esdt_bytes(&[]), //placeholder
                    token_nonce: 0,                                          //placeholder
                    amount: managed_biguint!(40_000),
                },
                attributes: LockedAssetTokenAttributesEx {
                    unlock_schedule: UnlockScheduleEx {
                        unlock_milestones: ManagedVec::from(vec![
                            UnlockMilestoneEx {
                                unlock_epoch: 0,
                                unlock_percent: 10_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 360,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 390,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 420,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 450,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 480,
                                unlock_percent: 15_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 510,
                                unlock_percent: 15_000,
                            },
                        ]),
                    },
                    is_merged: false,
                },
            };
            let token2 = LockedTokenEx::<DebugApi> {
                token_amount: EsdtTokenPayment {
                    token_identifier: TokenIdentifier::from_esdt_bytes(&[]), //placeholder
                    token_nonce: 0,                                          //placeholder
                    amount: managed_biguint!(60_000),
                },
                attributes: LockedAssetTokenAttributesEx {
                    unlock_schedule: UnlockScheduleEx {
                        unlock_milestones: ManagedVec::from(vec![
                            UnlockMilestoneEx {
                                unlock_epoch: 360,
                                unlock_percent: 16_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 390,
                                unlock_percent: 16_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 420,
                                unlock_percent: 17_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 450,
                                unlock_percent: 17_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 480,
                                unlock_percent: 17_000,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 510,
                                unlock_percent: 17_000,
                            },
                        ]),
                    },
                    is_merged: false,
                },
            };

            tokens.push(token1);
            tokens.push(token2);

            let result = sc.aggregated_unlock_schedule(&tokens);
            let result = result.unlock_milestones;

            let el = result.get(0);
            assert_eq!(el.unlock_epoch, 0);
            assert_eq!(el.unlock_percent, 4_000);

            let el = result.get(1);
            assert_eq!(el.unlock_epoch, 360);
            assert_eq!(el.unlock_percent, 15_600);

            let el = result.get(2);
            assert_eq!(el.unlock_epoch, 390);
            assert_eq!(el.unlock_percent, 15_600);

            let el = result.get(3);
            assert_eq!(el.unlock_epoch, 420);
            assert_eq!(el.unlock_percent, 16_200);

            let el = result.get(4);
            assert_eq!(el.unlock_epoch, 450);
            assert_eq!(el.unlock_percent, 16_200);

            let el = result.get(5);
            assert_eq!(el.unlock_epoch, 480);
            assert_eq!(el.unlock_percent, 16_200);

            let el = result.get(6);
            assert_eq!(el.unlock_epoch, 510);
            assert_eq!(el.unlock_percent, 16_200);
        })
        .assert_ok();
}

#[test]
fn test_aggregated_unlock_schedule() {
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let factory = blockchain_wrapper.create_sc_account(
        &rust_biguint!(0),
        None,
        factory::contract_obj,
        SC_WASM_PATH,
    );

    blockchain_wrapper
        .execute_query(&factory, |sc| {
            let mut tokens = ManagedVec::new();
            let token1 = LockedTokenEx::<DebugApi> {
                token_amount: EsdtTokenPayment {
                    token_identifier: TokenIdentifier::from_esdt_bytes(&[]), //placeholder
                    token_nonce: 0,                                          //placeholder
                    amount: managed_biguint!(608_212_266_882_971_044),
                },
                attributes: LockedAssetTokenAttributesEx {
                    unlock_schedule: UnlockScheduleEx {
                        unlock_milestones: ManagedVec::from(vec![
                            UnlockMilestoneEx {
                                unlock_epoch: 468,
                                unlock_percent: 14_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 348,
                                unlock_percent: 14_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 408,
                                unlock_percent: 14_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 318,
                                unlock_percent: 14_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 0,
                                unlock_percent: 16_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 378,
                                unlock_percent: 14_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 438,
                                unlock_percent: 14_00,
                            },
                        ]),
                    },
                    is_merged: false,
                },
            };
            let token2 = LockedTokenEx::<DebugApi> {
                token_amount: EsdtTokenPayment {
                    token_identifier: TokenIdentifier::from_esdt_bytes(&[]), //placeholder
                    token_nonce: 0,                                          //placeholder
                    amount: managed_biguint!(700_000_000_000_000_000),
                },
                attributes: LockedAssetTokenAttributesEx {
                    unlock_schedule: UnlockScheduleEx {
                        unlock_milestones: ManagedVec::from(vec![
                            UnlockMilestoneEx {
                                unlock_epoch: 378,
                                unlock_percent: 17_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 408,
                                unlock_percent: 17_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 438,
                                unlock_percent: 17_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 468,
                                unlock_percent: 16_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 498,
                                unlock_percent: 16_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 528,
                                unlock_percent: 16_00,
                            },
                        ]),
                    },
                    is_merged: false,
                },
            };

            tokens.push(token1);
            tokens.push(token2);

            let result = sc.aggregated_unlock_schedule(&tokens);
            let result = result.unlock_milestones;

            // if de-duplication fails, there will be 13 results (9 unique + 4 duplicates)
            assert_eq!(result.len(), 9);
        })
        .assert_ok();
}

#[test]
fn test_aggregated_unlock_schedule_with_1_offset() {
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let factory = blockchain_wrapper.create_sc_account(
        &rust_biguint!(0),
        None,
        factory::contract_obj,
        SC_WASM_PATH,
    );

    blockchain_wrapper
        .execute_query(&factory, |sc| {
            let mut tokens = ManagedVec::new();
            let token1 = LockedTokenEx::<DebugApi> {
                token_amount: EsdtTokenPayment {
                    token_identifier: TokenIdentifier::from_esdt_bytes(&[]), //placeholder
                    token_nonce: 0,                                          //placeholder
                    amount: managed_biguint!(608_212_266_882_971_044),
                },
                attributes: LockedAssetTokenAttributesEx {
                    unlock_schedule: UnlockScheduleEx {
                        unlock_milestones: ManagedVec::from(vec![
                            UnlockMilestoneEx {
                                unlock_epoch: 468,
                                unlock_percent: 14_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 348,
                                unlock_percent: 14_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 408,
                                unlock_percent: 14_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 318,
                                unlock_percent: 14_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 0,
                                unlock_percent: 16_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 378,
                                unlock_percent: 14_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 438,
                                unlock_percent: 14_00,
                            },
                        ]),
                    },
                    is_merged: false,
                },
            };
            let token2 = LockedTokenEx::<DebugApi> {
                token_amount: EsdtTokenPayment {
                    token_identifier: TokenIdentifier::from_esdt_bytes(&[]), //placeholder
                    token_nonce: 0,                                          //placeholder
                    amount: managed_biguint!(700_000_000_000_000_000),
                },
                attributes: LockedAssetTokenAttributesEx {
                    unlock_schedule: UnlockScheduleEx {
                        unlock_milestones: ManagedVec::from(vec![
                            UnlockMilestoneEx {
                                unlock_epoch: 378 + 1, //Notice the +1
                                unlock_percent: 17_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 408 + 1, //Notice the +1
                                unlock_percent: 17_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 438 + 1, //Notice the +1
                                unlock_percent: 17_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 468,
                                unlock_percent: 16_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 498,
                                unlock_percent: 16_00,
                            },
                            UnlockMilestoneEx {
                                unlock_epoch: 528,
                                unlock_percent: 16_00,
                            },
                        ]),
                    },
                    is_merged: false,
                },
            };

            tokens.push(token1);
            tokens.push(token2);

            let result = sc.aggregated_unlock_schedule(&tokens);
            let result = result.unlock_milestones;

            //In the end, the milestones with {$epoch, and ($epoch + 1)} should be placed under ($epoch + 1).

            assert_eq!(result.len(), 9);

            let el = result.get(0);
            assert_eq!(el.unlock_epoch, 0);

            let el = result.get(1);
            assert_eq!(el.unlock_epoch, 318);

            let el = result.get(2);
            assert_eq!(el.unlock_epoch, 348);

            let el = result.get(3);
            assert_eq!(el.unlock_epoch, 378 + 1);

            let el = result.get(4);
            assert_eq!(el.unlock_epoch, 408 + 1);

            let el = result.get(5);
            assert_eq!(el.unlock_epoch, 438 + 1);
        })
        .assert_ok();
}

#[test]
fn update_energy_after_old_token_unlock_test() {
    let _ = DebugApi::dummy();
    let rust_zero = rust_biguint!(0);
    let mut setup = FactorySetup::new(factory::contract_obj, energy_factory::contract_obj);

    let mut current_epoch = 1_441;
    setup.b_mock.set_block_epoch(current_epoch);

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

    let first_user = setup.first_user.clone();
    setup.b_mock.set_nft_balance(
        &first_user,
        LEGACY_LOCKED_TOKEN_ID,
        3, // higher random nonce to avoid nonce caching conflicts
        &rust_biguint!(USER_BALANCE),
        &old_token_attributes,
    );

    let mut user_energy_amount: multiversx_sc::types::BigUint<DebugApi> = managed_biguint!(0);
    user_energy_amount +=
        managed_biguint!(20_000) * USER_BALANCE * (first_unlock_epoch - current_epoch) / 100_000u32;
    user_energy_amount +=
        managed_biguint!(20_000) * USER_BALANCE * (second_unlock_epoch - current_epoch)
            / 100_000u32;
    user_energy_amount +=
        managed_biguint!(20_000) * USER_BALANCE * (third_unlock_epoch - current_epoch) / 100_000u32;
    user_energy_amount +=
        managed_biguint!(40_000) * USER_BALANCE * (forth_unlock_epoch - current_epoch) / 100_000u32;

    let expected_energy_vec = user_energy_amount.to_bytes_be().as_slice().to_vec();

    setup
        .b_mock
        .execute_tx(
            &first_user,
            &setup.energy_factory_wrapper,
            &rust_zero,
            |sc| {
                sc.set_paused(true);

                let mut users_energy = MultiValueEncoded::new();
                let user_energy = (
                    managed_address!(&first_user),
                    managed_biguint!(USER_BALANCE),
                    BigInt::from_signed_bytes_be(&expected_energy_vec),
                )
                    .into();
                users_energy.push(user_energy);
                sc.set_energy_for_old_tokens(users_energy);

                let expected_energy = Energy::new(
                    BigInt::from_signed_bytes_be(&expected_energy_vec),
                    1441,
                    managed_biguint!(USER_BALANCE),
                );
                let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
                assert_eq!(expected_energy, actual_energy);
                sc.set_paused(false);
            },
        )
        .assert_ok();

    current_epoch = 1_650;
    setup.b_mock.set_block_epoch(current_epoch);

    setup
        .b_mock
        .execute_esdt_transfer(
            &first_user,
            &setup.factory_wrapper,
            LEGACY_LOCKED_TOKEN_ID,
            3,
            &rust_biguint!(USER_BALANCE),
            |sc| {
                sc.set_paused(false);
                sc.unlock_assets();
            },
        )
        .assert_ok();

    // check user balance after unlocking the first two milestones
    let unlock_amount = USER_BALANCE / 100_000 * 20_000 + USER_BALANCE / 100_000 * 20_000;
    let remaining_locked_token_balace = USER_BALANCE - unlock_amount;

    setup.b_mock.check_esdt_balance(
        &first_user,
        BASE_ASSET_TOKEN_ID,
        &rust_biguint!(unlock_amount),
    );

    let mut new_unlock_milestones = ManagedVec::<DebugApi, UnlockMilestoneEx>::new();
    new_unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 33_333,
        unlock_epoch: third_unlock_epoch,
    });
    new_unlock_milestones.push(UnlockMilestoneEx {
        unlock_percent: 66_667,
        unlock_epoch: forth_unlock_epoch,
    });
    let new_locked_token_attributes = LockedAssetTokenAttributesEx {
        is_merged: false,
        unlock_schedule: UnlockScheduleEx {
            unlock_milestones: new_unlock_milestones,
        },
    };

    setup.b_mock.check_nft_balance(
        &first_user,
        LEGACY_LOCKED_TOKEN_ID,
        1, // new generated nonce (different from the initial randomly allocated nonce)
        &rust_biguint!(remaining_locked_token_balace),
        Some(&new_locked_token_attributes),
    );

    let mut final_user_energy_amount: multiversx_sc::types::BigUint<DebugApi> =
        managed_biguint!(0u64);
    final_user_energy_amount += managed_biguint!(33_333)
        * remaining_locked_token_balace
        * (third_unlock_epoch - current_epoch)
        / 100_000u32;
    final_user_energy_amount += managed_biguint!(66_667)
        * remaining_locked_token_balace
        * (forth_unlock_epoch - current_epoch)
        / 100_000u32; // 66_666 + 1 leftover

    let final_amount_vec = final_user_energy_amount.to_bytes_be().as_slice().to_vec();

    setup
        .b_mock
        .execute_query(&setup.energy_factory_wrapper, |sc| {
            let expected_energy = Energy::new(
                BigInt::from_signed_bytes_be(&final_amount_vec),
                current_epoch,
                managed_biguint!(remaining_locked_token_balace),
            );
            let actual_energy = sc.user_energy(&managed_address!(&first_user)).get();
            assert_eq!(expected_energy, actual_energy);
        })
        .assert_ok();
}
