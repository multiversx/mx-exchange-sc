#![allow(deprecated)]

use common_structs::{
    LockedAssetTokenAttributesEx, UnlockMilestone, UnlockMilestoneEx, UnlockScheduleEx,
};
use factory::locked_asset::LockedAssetModule;
use factory::*;
use metabonding_staking::MetabondingStaking;
use multiversx_sc::storage::mappers::StorageTokenWrapper;
use multiversx_sc::types::{Address, EsdtLocalRole, ManagedVec};
use multiversx_sc_modules::pause::PauseModule;
use multiversx_sc_scenario::whitebox_legacy::{TxResult, TxTokenTransfer};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, whitebox_legacy::*, DebugApi,
};

pub const METABONDING_STAKING_WASM_PATH: &str = "1.wasm";
pub const LOCKED_ASSET_FACTORY_WASM_PATH: &str = "2.wasm";
pub const ASSET_TOKEN_ID: &[u8] = b"MEX-123456";
pub const LOCKED_ASSET_TOKEN_ID: &[u8] = b"LKMEX-123456";

pub struct MetabondingStakingSetup<MetabondingStakingObjBuilder, LockedAssetFactoryObjBuilder>
where
    MetabondingStakingObjBuilder:
        'static + Copy + Fn() -> metabonding_staking::ContractObj<DebugApi>,
    LockedAssetFactoryObjBuilder: 'static + Copy + Fn() -> factory::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner_address: Address,
    pub user_address: Address,
    pub mbs_wrapper: ContractObjWrapper<
        metabonding_staking::ContractObj<DebugApi>,
        MetabondingStakingObjBuilder,
    >,
    pub laf_wrapper:
        ContractObjWrapper<factory::ContractObj<DebugApi>, LockedAssetFactoryObjBuilder>,
}

impl<MetabondingStakingObjBuilder, LockedAssetFactoryObjBuilder>
    MetabondingStakingSetup<MetabondingStakingObjBuilder, LockedAssetFactoryObjBuilder>
where
    MetabondingStakingObjBuilder:
        'static + Copy + Fn() -> metabonding_staking::ContractObj<DebugApi>,
    LockedAssetFactoryObjBuilder: 'static + Copy + Fn() -> factory::ContractObj<DebugApi>,
{
    pub fn new(
        mbs_builder: MetabondingStakingObjBuilder,
        laf_builder: LockedAssetFactoryObjBuilder,
    ) -> Self {
        let _ = DebugApi::dummy();

        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_addr = b_mock.create_user_account(&rust_zero);
        let user_addr = b_mock.create_user_account(&rust_zero);

        let laf_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            laf_builder,
            LOCKED_ASSET_FACTORY_WASM_PATH,
        );
        let mbs_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            mbs_builder,
            METABONDING_STAKING_WASM_PATH,
        );

        // set initial user balance

        // 100_000_000
        let attr1 = LockedAssetTokenAttributesEx::<DebugApi> {
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
        };
        // 1_000_000
        let attr2 = LockedAssetTokenAttributesEx::<DebugApi> {
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
        };

        b_mock.set_nft_balance(
            &user_addr,
            LOCKED_ASSET_TOKEN_ID,
            3,
            &rust_biguint!(100_000_000),
            &attr1,
        );
        b_mock.set_nft_balance(
            &user_addr,
            LOCKED_ASSET_TOKEN_ID,
            4,
            &rust_biguint!(1_000_000),
            &attr2,
        );

        // init Locked Asset Factory contract

        b_mock
            .execute_tx(&owner_addr, &laf_wrapper, &rust_zero, |sc| {
                let asset_token_id = managed_token_id!(ASSET_TOKEN_ID);
                let unlocked_percents = ManagedVec::from_single_item(UnlockMilestone {
                    unlock_epoch: 5,
                    unlock_percent: 100,
                });

                sc.init(asset_token_id, unlocked_percents.into());

                let locked_asset_token_id = managed_token_id!(LOCKED_ASSET_TOKEN_ID);
                sc.locked_asset_token().set_token_id(locked_asset_token_id);

                sc.set_paused(false);
            })
            .assert_ok();

        let locked_asset_token_roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ];
        b_mock.set_esdt_local_roles(
            laf_wrapper.address_ref(),
            LOCKED_ASSET_TOKEN_ID,
            &locked_asset_token_roles[..],
        );

        // init Metabonding Staking contract

        b_mock
            .execute_tx(&owner_addr, &mbs_wrapper, &rust_zero, |sc| {
                let locked_asset_token_id = managed_token_id!(LOCKED_ASSET_TOKEN_ID);
                let locked_asset_factory_addr = managed_address!(laf_wrapper.address_ref());

                sc.init(locked_asset_token_id, locked_asset_factory_addr);
            })
            .assert_ok();

        Self {
            b_mock,
            laf_wrapper,
            mbs_wrapper,
            owner_address: owner_addr,
            user_address: user_addr,
        }
    }

    pub fn call_stake_locked_asset(&mut self, token_nonce: u64, amount: u64) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            &self.user_address,
            &self.mbs_wrapper,
            LOCKED_ASSET_TOKEN_ID,
            token_nonce,
            &rust_biguint!(amount),
            |sc| {
                sc.stake_locked_asset();
            },
        )
    }

    pub fn call_stake_locked_asset_multiple(&mut self, payments: &[TxTokenTransfer]) -> TxResult {
        self.b_mock.execute_esdt_multi_transfer(
            &self.user_address,
            &self.mbs_wrapper,
            payments,
            |sc| {
                sc.stake_locked_asset();
            },
        )
    }

    pub fn call_unstake(&mut self, amount: u64) -> TxResult {
        self.b_mock.execute_tx(
            &self.user_address,
            &self.mbs_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.unstake(managed_biguint!(amount));
            },
        )
    }

    pub fn call_unbond(&mut self) -> TxResult {
        self.b_mock.execute_tx(
            &self.user_address,
            &self.mbs_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.unbond();
            },
        )
    }
}
