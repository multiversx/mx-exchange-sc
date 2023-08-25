#![allow(deprecated)]

use multiversx_sc::{
    codec::multi_types::OptionalValue,
    storage::mappers::StorageTokenWrapper,
    types::{Address, BigInt, EsdtLocalRole, MultiValueEncoded},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    whitebox_legacy::TxResult, whitebox_legacy::*, DebugApi,
};

use energy_factory::{energy::EnergyModule, SimpleLockEnergy};
use energy_query::{Energy, EnergyQueryModule};
use fees_collector::{config::ConfigModule, fees_accumulation::FeesAccumulationModule, *};
use locking_module::lock_with_energy_module::LockWithEnergyModule;
use multiversx_sc_modules::pause::PauseModule;
use sc_whitelist_module::SCWhitelistModule;
use simple_lock::locked_token::{LockedTokenAttributes, LockedTokenModule};
use week_timekeeping::{Week, WeekTimekeepingModule, EPOCHS_IN_WEEK};

pub const INIT_EPOCH: u64 = 5;
pub const EPOCHS_IN_YEAR: u64 = 360;
pub const USER_BALANCE: u64 = 1_000_000_000_000_000_000;

pub static LOCK_OPTIONS: &[u64] = &[EPOCHS_IN_YEAR, 2 * EPOCHS_IN_YEAR, 4 * EPOCHS_IN_YEAR];
pub static FIRST_TOKEN_ID: &[u8] = b"FIRST-123456";
pub static SECOND_TOKEN_ID: &[u8] = b"SECOND-123456";
pub static BASE_ASSET_TOKEN_ID: &[u8] = b"MEX-123456";
pub static LOCKED_TOKEN_ID: &[u8] = b"LOCKED-123456";
pub static LEGACY_LOCKED_TOKEN_ID: &[u8] = b"LEGACY-123456";
pub static PENALTY_PERCENTAGES: &[u64] = &[4_000, 6_000, 8_000];

pub struct FeesCollectorSetup<FeesCollectorObjBuilder, EnergyFactoryObjBuilder>
where
    FeesCollectorObjBuilder: 'static + Copy + Fn() -> fees_collector::ContractObj<DebugApi>,
    EnergyFactoryObjBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner_address: Address,
    pub depositor_address: Address,
    pub fc_wrapper:
        ContractObjWrapper<fees_collector::ContractObj<DebugApi>, FeesCollectorObjBuilder>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory::ContractObj<DebugApi>, EnergyFactoryObjBuilder>,
    pub current_epoch: u64,
}

impl<FeesCollectorObjBuilder, EnergyFactoryObjBuilder>
    FeesCollectorSetup<FeesCollectorObjBuilder, EnergyFactoryObjBuilder>
where
    FeesCollectorObjBuilder: 'static + Copy + Fn() -> fees_collector::ContractObj<DebugApi>,
    EnergyFactoryObjBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
{
    pub fn new(
        fc_builder: FeesCollectorObjBuilder,
        energy_factory_builder: EnergyFactoryObjBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_address = b_mock.create_user_account(&rust_zero);
        let depositor_address = b_mock.create_user_account(&rust_zero);
        let fc_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_address),
            fc_builder,
            "fees collector path",
        );
        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_address),
            energy_factory_builder,
            "energy factory path",
        );

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

        // set fees collector roles
        b_mock.set_esdt_local_roles(
            fc_wrapper.address_ref(),
            LOCKED_TOKEN_ID,
            &[EsdtLocalRole::NftBurn],
        );

        b_mock.set_esdt_balance(
            &depositor_address,
            FIRST_TOKEN_ID,
            &rust_biguint!(USER_BALANCE * 2),
        );
        b_mock.set_esdt_balance(
            &depositor_address,
            SECOND_TOKEN_ID,
            &rust_biguint!(USER_BALANCE * 2),
        );

        let _ = DebugApi::dummy();

        b_mock.set_nft_balance(
            &depositor_address,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(USER_BALANCE * 2),
            &LockedTokenAttributes::<DebugApi> {
                original_token_id: managed_token_id_wrapped!(BASE_ASSET_TOKEN_ID),
                original_token_nonce: 1,
                unlock_epoch: 100,
            },
        );

        b_mock.set_block_epoch(INIT_EPOCH);

        // setup energy factory
        b_mock
            .execute_tx(&owner_address, &energy_factory_wrapper, &rust_zero, |sc| {
                let mut lock_options = MultiValueEncoded::new();
                for (option, penalty) in LOCK_OPTIONS.iter().zip(PENALTY_PERCENTAGES.iter()) {
                    lock_options.push((*option, *penalty).into());
                }
                sc.init(
                    managed_token_id!(BASE_ASSET_TOKEN_ID),
                    managed_token_id!(LEGACY_LOCKED_TOKEN_ID),
                    managed_address!(energy_factory_wrapper.address_ref()),
                    0,
                    lock_options,
                );

                sc.locked_token()
                    .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
                sc.set_paused(false);
                sc.add_sc_address_to_whitelist(managed_address!(fc_wrapper.address_ref()));
            })
            .assert_ok();

        b_mock
            .execute_tx(&owner_address, &fc_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_token_id!(LOCKED_TOKEN_ID),
                    managed_address!(energy_factory_wrapper.address_ref()),
                );

                let _ = sc
                    .known_contracts()
                    .insert(managed_address!(&depositor_address));

                let mut tokens = MultiValueEncoded::new();
                tokens.push(managed_token_id!(FIRST_TOKEN_ID));
                tokens.push(managed_token_id!(SECOND_TOKEN_ID));
                tokens.push(managed_token_id!(LOCKED_TOKEN_ID));

                sc.add_known_tokens(tokens);

                sc.set_energy_factory_address(managed_address!(
                    energy_factory_wrapper.address_ref()
                ));
                sc.set_locking_sc_address(managed_address!(energy_factory_wrapper.address_ref()));
                sc.set_lock_epochs(LOCK_OPTIONS[2]);
            })
            .assert_ok();

        FeesCollectorSetup {
            b_mock,
            owner_address,
            depositor_address,
            fc_wrapper,
            energy_factory_wrapper,
            current_epoch: INIT_EPOCH,
        }
    }

    pub fn advance_week(&mut self) {
        self.current_epoch += EPOCHS_IN_WEEK;
        self.b_mock.set_block_epoch(self.current_epoch);
    }

    pub fn get_current_week(&mut self) -> Week {
        let mut result = 0;
        self.b_mock
            .execute_query(&self.fc_wrapper, |sc| result = sc.get_current_week())
            .assert_ok();

        result
    }

    pub fn deposit(&mut self, token: &[u8], amount: u64) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            &self.depositor_address,
            &self.fc_wrapper,
            token,
            0,
            &rust_biguint!(amount),
            |sc| {
                sc.deposit_swap_fees();
            },
        )
    }

    pub fn deposit_locked_tokens(&mut self, token: &[u8], nonce: u64, amount: u64) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            &self.depositor_address,
            &self.fc_wrapper,
            token,
            nonce,
            &rust_biguint!(amount),
            |sc| {
                sc.deposit_swap_fees();
            },
        )
    }

    pub fn claim(&mut self, user: &Address) -> TxResult {
        self.b_mock
            .execute_tx(user, &self.fc_wrapper, &rust_biguint!(0), |sc| {
                let _ = sc.claim_rewards(OptionalValue::None);
            })
    }

    pub fn set_energy(&mut self, user: &Address, total_locked_tokens: u64, energy_amount: u64) {
        let current_epoch = self.current_epoch;
        self.b_mock
            .execute_tx(
                user,
                &self.energy_factory_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.user_energy(&managed_address!(user)).set(&Energy::new(
                        BigInt::from(managed_biguint!(energy_amount)),
                        current_epoch,
                        managed_biguint!(total_locked_tokens),
                    ));
                },
            )
            .assert_ok();
    }
}
