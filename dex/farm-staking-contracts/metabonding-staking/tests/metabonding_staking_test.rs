use common_structs::UnlockMilestone;
use elrond_wasm::elrond_codec::multi_types::OptionalValue;
use elrond_wasm::types::{Address, EsdtLocalRole, ManagedVec};
use elrond_wasm_debug::tx_mock::{TxContextStack, TxInputESDT};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, testing_framework::*,
    DebugApi,
};
use factory::locked_asset::LockedAssetModule;
use factory::*;
use metabonding_staking::MetabondingStaking;

type RustBigUint = num_bigint::BigUint;

const METABONDING_STAKING_WASM_PATH: &'static str = "1.wasm";
const LOCKED_ASSET_FACTORY_WASM_PATH: &'static str = "2.wasm";
const ASSET_TOKEN_ID: &[u8] = b"MEX-123456";
const LOCKED_ASSET_TOKEN_ID: &[u8] = b"LKMEX-123456";

struct MetabondingStakingSetup<MetabondingStakingObjBuilder, LockedAssetFactoryObjBuilder>
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
                sc.locked_asset_token_id().set(&locked_asset_token_id);
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
}
