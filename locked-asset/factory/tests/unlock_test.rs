use common_structs::{LockedAssetTokenAttributes, UnlockMilestone, UnlockSchedule};
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

    blockchain_wrapper.execute_query(&factory, |sc| {
        let mut tokens = ManagedVec::new();
        let token1 = LockedToken::<DebugApi> {
            token_amount: EsdtTokenPayment {
                token_type: EsdtTokenType::NonFungible,    //placeholder
                token_identifier: TokenIdentifier::egld(), //placeholder
                token_nonce: 0,                            //placeholder
                amount: managed_biguint!(100_000_000),
            },
            attributes: LockedAssetTokenAttributes {
                unlock_schedule: UnlockSchedule {
                    unlock_milestones: ManagedVec::from(vec![
                        UnlockMilestone {
                            unlock_epoch: 0,
                            unlock_percent: 10,
                        },
                        UnlockMilestone {
                            unlock_epoch: 360,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 390,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 420,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 450,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 480,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 510,
                            unlock_percent: 15,
                        },
                    ]),
                },
                is_merged: false,
            },
        };
        let token2 = LockedToken::<DebugApi> {
            token_amount: EsdtTokenPayment {
                token_type: EsdtTokenType::NonFungible,    //placeholder
                token_identifier: TokenIdentifier::egld(), //placeholder
                token_nonce: 0,                            //placeholder
                amount: managed_biguint!(1_000_000),
            },
            attributes: LockedAssetTokenAttributes {
                unlock_schedule: UnlockSchedule {
                    unlock_milestones: ManagedVec::from(vec![
                        UnlockMilestone {
                            unlock_epoch: 360,
                            unlock_percent: 16,
                        },
                        UnlockMilestone {
                            unlock_epoch: 390,
                            unlock_percent: 16,
                        },
                        UnlockMilestone {
                            unlock_epoch: 420,
                            unlock_percent: 17,
                        },
                        UnlockMilestone {
                            unlock_epoch: 450,
                            unlock_percent: 17,
                        },
                        UnlockMilestone {
                            unlock_epoch: 480,
                            unlock_percent: 17,
                        },
                        UnlockMilestone {
                            unlock_epoch: 510,
                            unlock_percent: 17,
                        },
                    ]),
                },
                is_merged: false,
            },
        };

        tokens.push(token1);
        tokens.push(token2);

        let result = sc.aggregated_unlock_schedule(&tokens).unwrap();
        let result = result.unlock_milestones;

        let el = result.get(0).unwrap();
        assert_eq!(el.unlock_epoch, 0);
        assert_eq!(el.unlock_percent, 10);

        let el = result.get(1).unwrap();
        assert_eq!(el.unlock_epoch, 360);
        assert_eq!(el.unlock_percent, 15);

        let el = result.get(2).unwrap();
        assert_eq!(el.unlock_epoch, 390);
        assert_eq!(el.unlock_percent, 15);

        let el = result.get(3).unwrap();
        assert_eq!(el.unlock_epoch, 420);
        assert_eq!(el.unlock_percent, 15);

        let el = result.get(4).unwrap();
        assert_eq!(el.unlock_epoch, 450);
        assert_eq!(el.unlock_percent, 15);

        let el = result.get(5).unwrap();
        assert_eq!(el.unlock_epoch, 480);
        assert_eq!(el.unlock_percent, 15);

        let el = result.get(6).unwrap();
        assert_eq!(el.unlock_epoch, 510);
        assert_eq!(el.unlock_percent, 15);
    });
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

    blockchain_wrapper.execute_query(&factory, |sc| {
        let mut tokens = ManagedVec::new();
        let token1 = LockedToken::<DebugApi> {
            token_amount: EsdtTokenPayment {
                token_type: EsdtTokenType::NonFungible,    //placeholder
                token_identifier: TokenIdentifier::egld(), //placeholder
                token_nonce: 0,                            //placeholder
                amount: managed_biguint!(1_000_000),
            },
            attributes: LockedAssetTokenAttributes {
                unlock_schedule: UnlockSchedule {
                    unlock_milestones: ManagedVec::from(vec![
                        UnlockMilestone {
                            unlock_epoch: 0,
                            unlock_percent: 10,
                        },
                        UnlockMilestone {
                            unlock_epoch: 360,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 390,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 420,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 450,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 480,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 510,
                            unlock_percent: 15,
                        },
                    ]),
                },
                is_merged: false,
            },
        };
        let token2 = LockedToken::<DebugApi> {
            token_amount: EsdtTokenPayment {
                token_type: EsdtTokenType::NonFungible,    //placeholder
                token_identifier: TokenIdentifier::egld(), //placeholder
                token_nonce: 0,                            //placeholder
                amount: managed_biguint!(100_000_000),
            },
            attributes: LockedAssetTokenAttributes {
                unlock_schedule: UnlockSchedule {
                    unlock_milestones: ManagedVec::from(vec![
                        UnlockMilestone {
                            unlock_epoch: 360,
                            unlock_percent: 16,
                        },
                        UnlockMilestone {
                            unlock_epoch: 390,
                            unlock_percent: 16,
                        },
                        UnlockMilestone {
                            unlock_epoch: 420,
                            unlock_percent: 17,
                        },
                        UnlockMilestone {
                            unlock_epoch: 450,
                            unlock_percent: 17,
                        },
                        UnlockMilestone {
                            unlock_epoch: 480,
                            unlock_percent: 17,
                        },
                        UnlockMilestone {
                            unlock_epoch: 510,
                            unlock_percent: 17,
                        },
                    ]),
                },
                is_merged: false,
            },
        };

        tokens.push(token1);
        tokens.push(token2);

        let result = sc.aggregated_unlock_schedule(&tokens).unwrap();
        let result = result.unlock_milestones;

        let el = result.get(0).unwrap();
        assert_eq!(el.unlock_epoch, 360);
        assert_eq!(el.unlock_percent, 16);

        let el = result.get(1).unwrap();
        assert_eq!(el.unlock_epoch, 390);
        assert_eq!(el.unlock_percent, 16);

        let el = result.get(2).unwrap();
        assert_eq!(el.unlock_epoch, 420);
        assert_eq!(el.unlock_percent, 17);

        let el = result.get(3).unwrap();
        assert_eq!(el.unlock_epoch, 450);
        assert_eq!(el.unlock_percent, 17);

        let el = result.get(4).unwrap();
        assert_eq!(el.unlock_epoch, 480);
        assert_eq!(el.unlock_percent, 17);

        let el = result.get(5).unwrap();
        assert_eq!(el.unlock_epoch, 510);
        assert_eq!(el.unlock_percent, 17);
    });
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

    blockchain_wrapper.execute_query(&factory, |sc| {
        let mut tokens = ManagedVec::new();
        let token1 = LockedToken::<DebugApi> {
            token_amount: EsdtTokenPayment {
                token_type: EsdtTokenType::NonFungible,    //placeholder
                token_identifier: TokenIdentifier::egld(), //placeholder
                token_nonce: 0,                            //placeholder
                amount: managed_biguint!(60_000),
            },
            attributes: LockedAssetTokenAttributes {
                unlock_schedule: UnlockSchedule {
                    unlock_milestones: ManagedVec::from(vec![
                        UnlockMilestone {
                            unlock_epoch: 0,
                            unlock_percent: 10,
                        },
                        UnlockMilestone {
                            unlock_epoch: 360,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 390,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 420,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 450,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 480,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 510,
                            unlock_percent: 15,
                        },
                    ]),
                },
                is_merged: false,
            },
        };
        let token2 = LockedToken::<DebugApi> {
            token_amount: EsdtTokenPayment {
                token_type: EsdtTokenType::NonFungible,    //placeholder
                token_identifier: TokenIdentifier::egld(), //placeholder
                token_nonce: 0,                            //placeholder
                amount: managed_biguint!(40_000),
            },
            attributes: LockedAssetTokenAttributes {
                unlock_schedule: UnlockSchedule {
                    unlock_milestones: ManagedVec::from(vec![
                        UnlockMilestone {
                            unlock_epoch: 360,
                            unlock_percent: 16,
                        },
                        UnlockMilestone {
                            unlock_epoch: 390,
                            unlock_percent: 16,
                        },
                        UnlockMilestone {
                            unlock_epoch: 420,
                            unlock_percent: 17,
                        },
                        UnlockMilestone {
                            unlock_epoch: 450,
                            unlock_percent: 17,
                        },
                        UnlockMilestone {
                            unlock_epoch: 480,
                            unlock_percent: 17,
                        },
                        UnlockMilestone {
                            unlock_epoch: 510,
                            unlock_percent: 17,
                        },
                    ]),
                },
                is_merged: false,
            },
        };

        tokens.push(token1);
        tokens.push(token2);

        let result = sc.aggregated_unlock_schedule(&tokens).unwrap();
        let result = result.unlock_milestones;

        let el = result.get(0).unwrap();
        assert_eq!(el.unlock_epoch, 0);
        assert_eq!(el.unlock_percent, 6);

        let el = result.get(1).unwrap();
        assert_eq!(el.unlock_epoch, 360);
        assert_eq!(el.unlock_percent, 15);

        let el = result.get(2).unwrap();
        assert_eq!(el.unlock_epoch, 390);
        assert_eq!(el.unlock_percent, 15);

        let el = result.get(3).unwrap();
        assert_eq!(el.unlock_epoch, 420);
        assert_eq!(el.unlock_percent, 16);

        let el = result.get(4).unwrap();
        assert_eq!(el.unlock_epoch, 450);
        assert_eq!(el.unlock_percent, 16);

        let el = result.get(5).unwrap();
        assert_eq!(el.unlock_epoch, 480);
        assert_eq!(el.unlock_percent, 16);

        let el = result.get(6).unwrap();
        assert_eq!(el.unlock_epoch, 510);
        assert_eq!(el.unlock_percent, 16);
    });
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

    blockchain_wrapper.execute_query(&factory, |sc| {
        let mut tokens = ManagedVec::new();
        let token1 = LockedToken::<DebugApi> {
            token_amount: EsdtTokenPayment {
                token_type: EsdtTokenType::NonFungible,    //placeholder
                token_identifier: TokenIdentifier::egld(), //placeholder
                token_nonce: 0,                            //placeholder
                amount: managed_biguint!(40_000),
            },
            attributes: LockedAssetTokenAttributes {
                unlock_schedule: UnlockSchedule {
                    unlock_milestones: ManagedVec::from(vec![
                        UnlockMilestone {
                            unlock_epoch: 0,
                            unlock_percent: 10,
                        },
                        UnlockMilestone {
                            unlock_epoch: 360,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 390,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 420,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 450,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 480,
                            unlock_percent: 15,
                        },
                        UnlockMilestone {
                            unlock_epoch: 510,
                            unlock_percent: 15,
                        },
                    ]),
                },
                is_merged: false,
            },
        };
        let token2 = LockedToken::<DebugApi> {
            token_amount: EsdtTokenPayment {
                token_type: EsdtTokenType::NonFungible,    //placeholder
                token_identifier: TokenIdentifier::egld(), //placeholder
                token_nonce: 0,                            //placeholder
                amount: managed_biguint!(60_000),
            },
            attributes: LockedAssetTokenAttributes {
                unlock_schedule: UnlockSchedule {
                    unlock_milestones: ManagedVec::from(vec![
                        UnlockMilestone {
                            unlock_epoch: 360,
                            unlock_percent: 16,
                        },
                        UnlockMilestone {
                            unlock_epoch: 390,
                            unlock_percent: 16,
                        },
                        UnlockMilestone {
                            unlock_epoch: 420,
                            unlock_percent: 17,
                        },
                        UnlockMilestone {
                            unlock_epoch: 450,
                            unlock_percent: 17,
                        },
                        UnlockMilestone {
                            unlock_epoch: 480,
                            unlock_percent: 17,
                        },
                        UnlockMilestone {
                            unlock_epoch: 510,
                            unlock_percent: 17,
                        },
                    ]),
                },
                is_merged: false,
            },
        };

        tokens.push(token1);
        tokens.push(token2);

        let result = sc.aggregated_unlock_schedule(&tokens).unwrap();
        let result = result.unlock_milestones;

        let el = result.get(0).unwrap();
        assert_eq!(el.unlock_epoch, 0);
        assert_eq!(el.unlock_percent, 4);

        let el = result.get(1).unwrap();
        assert_eq!(el.unlock_epoch, 360);
        assert_eq!(el.unlock_percent, 16);

        let el = result.get(2).unwrap();
        assert_eq!(el.unlock_epoch, 390);
        assert_eq!(el.unlock_percent, 16);

        let el = result.get(3).unwrap();
        assert_eq!(el.unlock_epoch, 420);
        assert_eq!(el.unlock_percent, 16);

        let el = result.get(4).unwrap();
        assert_eq!(el.unlock_epoch, 450);
        assert_eq!(el.unlock_percent, 16);

        let el = result.get(5).unwrap();
        assert_eq!(el.unlock_epoch, 480);
        assert_eq!(el.unlock_percent, 16);

        let el = result.get(6).unwrap();
        assert_eq!(el.unlock_epoch, 510);
        assert_eq!(el.unlock_percent, 16);
    });
}
