#![allow(dead_code)]
#![allow(deprecated)]

use common_structs::UnlockMilestone;
use energy_factory::SimpleLockEnergy;
use factory::{
    locked_asset::LockedAssetModule, migration::LockedTokenMigrationModule, LockedAssetFactory,
};
use multiversx_sc::{
    storage::mappers::StorageTokenWrapper,
    types::{Address, EsdtLocalRole, MultiValueEncoded},
};
use multiversx_sc_scenario::{
    managed_address, managed_token_id, rust_biguint,
    whitebox_legacy::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};
use simple_lock::locked_token::LockedTokenModule;

pub const EPOCHS_IN_YEAR: u64 = 360;
pub const EPOCHS_IN_WEEK: u64 = 7;
pub const USER_BALANCE: u64 = 1_000_000_000_000_000_000;

pub static BASE_ASSET_TOKEN_ID: &[u8] = b"MEX-123456";
pub static LOCKED_TOKEN_ID: &[u8] = b"LOCKED-123456";
pub static LEGACY_LOCKED_TOKEN_ID: &[u8] = b"LEGACY-123456";

pub static LOCK_OPTIONS: &[u64] = &[EPOCHS_IN_YEAR, 2 * EPOCHS_IN_YEAR, 4 * EPOCHS_IN_YEAR]; // 1, 2 or 4 years
pub static PENALTY_PERCENTAGES: &[u64] = &[4_000, 6_000, 8_000];

pub struct FactorySetup<FactoryBuilder, EnergyFactoryBuilder>
where
    FactoryBuilder: 'static + Copy + Fn() -> factory::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub first_user: Address,
    pub factory_wrapper: ContractObjWrapper<factory::ContractObj<DebugApi>, FactoryBuilder>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory::ContractObj<DebugApi>, EnergyFactoryBuilder>,
}

impl<FactoryBuilder, EnergyFactoryBuilder> FactorySetup<FactoryBuilder, EnergyFactoryBuilder>
where
    FactoryBuilder: 'static + Copy + Fn() -> factory::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
{
    pub fn new(
        factory_builder: FactoryBuilder,
        energy_factory_builder: EnergyFactoryBuilder,
    ) -> Self {
        let _ = DebugApi::dummy();
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let first_user = b_mock.create_user_account(&rust_zero);
        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            energy_factory_builder,
            "energy factory",
        );
        let factory_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), factory_builder, "factory");

        b_mock
            .execute_tx(&owner, &energy_factory_wrapper, &rust_zero, |sc| {
                let mut lock_options = MultiValueEncoded::new();
                for (option, penalty) in LOCK_OPTIONS.iter().zip(PENALTY_PERCENTAGES.iter()) {
                    lock_options.push((*option, *penalty).into());
                }
                sc.init(
                    managed_token_id!(BASE_ASSET_TOKEN_ID),
                    managed_token_id!(LEGACY_LOCKED_TOKEN_ID),
                    managed_address!(factory_wrapper.address_ref()),
                    0,
                    lock_options,
                );

                sc.locked_token()
                    .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            })
            .assert_ok();

        b_mock
            .execute_tx(&owner, &factory_wrapper, &rust_zero, |sc| {
                let mut default_unlock_period = MultiValueEncoded::new();
                default_unlock_period.push(UnlockMilestone {
                    unlock_epoch: 0,
                    unlock_percent: 100,
                });

                sc.init(
                    managed_token_id!(BASE_ASSET_TOKEN_ID),
                    default_unlock_period,
                );
                sc.new_factory_address()
                    .set(managed_address!(energy_factory_wrapper.address_ref()));
                sc.locked_asset_token()
                    .set_token_id(managed_token_id!(LEGACY_LOCKED_TOKEN_ID));
            })
            .assert_ok();

        // set energy factory roles
        b_mock.set_esdt_local_roles(
            energy_factory_wrapper.address_ref(),
            BASE_ASSET_TOKEN_ID,
            &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
        );
        b_mock.set_esdt_local_roles(
            energy_factory_wrapper.address_ref(),
            LOCKED_TOKEN_ID,
            &[
                EsdtLocalRole::NftCreate,
                EsdtLocalRole::NftAddQuantity,
                EsdtLocalRole::NftBurn,
                EsdtLocalRole::Transfer,
            ],
        );
        b_mock.set_esdt_local_roles(
            energy_factory_wrapper.address_ref(),
            LEGACY_LOCKED_TOKEN_ID,
            &[EsdtLocalRole::NftBurn],
        );

        // set factory roles
        b_mock.set_esdt_local_roles(
            factory_wrapper.address_ref(),
            BASE_ASSET_TOKEN_ID,
            &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
        );
        b_mock.set_esdt_local_roles(
            factory_wrapper.address_ref(),
            LEGACY_LOCKED_TOKEN_ID,
            &[
                EsdtLocalRole::NftCreate,
                EsdtLocalRole::NftAddQuantity,
                EsdtLocalRole::NftBurn,
                EsdtLocalRole::Transfer,
            ],
        );

        Self {
            b_mock,
            owner,
            first_user,
            factory_wrapper,
            energy_factory_wrapper,
        }
    }
}
