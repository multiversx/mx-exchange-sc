use common_structs::{LockedAssetTokenAttributesEx, UnlockMilestoneEx, UnlockScheduleEx};
use elrond_wasm::types::{EsdtTokenPayment, EsdtTokenType, ManagedVec};
use elrond_wasm_debug::testing_framework::BigUint;
use elrond_wasm_debug::{managed_biguint, rust_biguint, testing_framework::*, DebugApi};

const SC_WASM_PATH: &'static str = "output/factory.wasm";

use factory::locked_asset::*;
use factory::locked_asset_token_merge::*;

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
                    token_type: EsdtTokenType::NonFungible,    //placeholder
                    token_identifier: TokenIdentifier::egld(), //placeholder
                    token_nonce: 0,                            //placeholder
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
                    token_type: EsdtTokenType::NonFungible,    //placeholder
                    token_identifier: TokenIdentifier::egld(), //placeholder
                    token_nonce: 0,                            //placeholder
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
                    token_type: EsdtTokenType::NonFungible,    //placeholder
                    token_identifier: TokenIdentifier::egld(), //placeholder
                    token_nonce: 0,                            //placeholder
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
                    token_type: EsdtTokenType::NonFungible,    //placeholder
                    token_identifier: TokenIdentifier::egld(), //placeholder
                    token_nonce: 0,                            //placeholder
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
                    token_type: EsdtTokenType::NonFungible,    //placeholder
                    token_identifier: TokenIdentifier::egld(), //placeholder
                    token_nonce: 0,                            //placeholder
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
                    token_type: EsdtTokenType::NonFungible,    //placeholder
                    token_identifier: TokenIdentifier::egld(), //placeholder
                    token_nonce: 0,                            //placeholder
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
                    token_type: EsdtTokenType::NonFungible,    //placeholder
                    token_identifier: TokenIdentifier::egld(), //placeholder
                    token_nonce: 0,                            //placeholder
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
                    token_type: EsdtTokenType::NonFungible,    //placeholder
                    token_identifier: TokenIdentifier::egld(), //placeholder
                    token_nonce: 0,                            //placeholder
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
                    token_type: EsdtTokenType::NonFungible,    //placeholder
                    token_identifier: TokenIdentifier::egld(), //placeholder
                    token_nonce: 0,                            //placeholder
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
                    token_type: EsdtTokenType::NonFungible,    //placeholder
                    token_identifier: TokenIdentifier::egld(), //placeholder
                    token_nonce: 0,                            //placeholder
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

            print!("{:?}", result);

            // if de-duplication fails, there will be 13 results (9 unique + 4 duplicates)
            assert_eq!(result.len(), 9);
        })
        .assert_ok();
}
