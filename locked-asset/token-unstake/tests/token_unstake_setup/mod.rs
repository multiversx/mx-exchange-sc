#![allow(dead_code)]
#![allow(deprecated)]

pub mod fees_collector_mock;

use energy_factory::{
    energy::EnergyModule, unlock_with_penalty::UnlockWithPenaltyModule, unstake::UnstakeModule,
    SimpleLockEnergy,
};
use fees_collector_mock::*;
use multiversx_sc::{
    codec::multi_types::OptionalValue,
    storage::mappers::StorageTokenWrapper,
    types::{Address, EsdtLocalRole, MultiValueEncoded},
};
use multiversx_sc_modules::pause::PauseModule;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    whitebox_legacy::TxResult,
    whitebox_legacy::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};
use simple_lock::locked_token::LockedTokenModule;
use token_unstake::{
    cancel_unstake::CancelUnstakeModule, unbond_tokens::UnbondTokensModule, TokenUnstakeModule,
};

pub const EPOCHS_IN_YEAR: u64 = 360;
pub const EPOCHS_IN_WEEK: u64 = 7;
pub const USER_BALANCE: u64 = 1_000_000_000_000_000_000;

pub static BASE_ASSET_TOKEN_ID: &[u8] = b"MEX-123456";
pub static LOCKED_TOKEN_ID: &[u8] = b"LOCKED-123456";
pub static LEGACY_LOCKED_TOKEN_ID: &[u8] = b"LEGACY-123456";

pub static LOCK_OPTIONS: &[u64] = &[EPOCHS_IN_YEAR, 2 * EPOCHS_IN_YEAR, 4 * EPOCHS_IN_YEAR]; // 1, 2 or 4 years
pub static PENALTY_PERCENTAGES: &[u64] = &[4_000, 6_000, 8_000];

pub const FEES_BURN_PERCENTAGE: u64 = 5_000; // 50%
pub const UNBOND_EPOCHS: u64 = 10;

pub struct TokenUnstakeSetup<EnergyFactoryBuilder, UnstakeScBuilder>
where
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    UnstakeScBuilder: 'static + Copy + Fn() -> token_unstake::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub first_user: Address,
    pub second_user: Address,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory::ContractObj<DebugApi>, EnergyFactoryBuilder>,
    pub unstake_sc_wrapper:
        ContractObjWrapper<token_unstake::ContractObj<DebugApi>, UnstakeScBuilder>,
    pub fees_collector_mock: Address,
}

impl<EnergyFactoryBuilder, UnstakeScBuilder>
    TokenUnstakeSetup<EnergyFactoryBuilder, UnstakeScBuilder>
where
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    UnstakeScBuilder: 'static + Copy + Fn() -> token_unstake::ContractObj<DebugApi>,
{
    pub fn new(
        energy_factory_builder: EnergyFactoryBuilder,
        unstake_sc_builder: UnstakeScBuilder,
    ) -> Self {
        let _ = DebugApi::dummy();
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let first_user = b_mock.create_user_account(&rust_zero);
        let second_user = b_mock.create_user_account(&rust_zero);
        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            energy_factory_builder,
            "simple lock energy",
        );
        let unstake_sc_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            unstake_sc_builder,
            "unstake token",
        );
        let fees_collector_mock = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            FeesCollectorMock::new,
            "fees collector mock",
        );

        // setup energy factory
        b_mock
            .execute_tx(&owner, &energy_factory_wrapper, &rust_zero, |sc| {
                let mut lock_options = MultiValueEncoded::new();
                for (option, penalty) in LOCK_OPTIONS.iter().zip(PENALTY_PERCENTAGES.iter()) {
                    lock_options.push((*option, *penalty).into());
                }

                // fees_collector_mock address used twice, as we don't test migration here
                // migration is tested in old_to_new_locked_token_migration_test.rs
                sc.init(
                    managed_token_id!(BASE_ASSET_TOKEN_ID),
                    managed_token_id!(LEGACY_LOCKED_TOKEN_ID),
                    managed_address!(unstake_sc_wrapper.address_ref()),
                    0,
                    lock_options,
                );

                sc.locked_token()
                    .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
                sc.set_paused(false);
                sc.set_token_unstake_address(managed_address!(unstake_sc_wrapper.address_ref()));
            })
            .assert_ok();

        // setup unstake sc
        b_mock
            .execute_tx(&owner, &unstake_sc_wrapper, &rust_zero, |sc| {
                sc.init(
                    UNBOND_EPOCHS,
                    managed_address!(energy_factory_wrapper.address_ref()),
                    FEES_BURN_PERCENTAGE,
                    managed_address!(fees_collector_mock.address_ref()),
                );
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

        // set unbond sc roles
        b_mock.set_esdt_local_roles(
            unstake_sc_wrapper.address_ref(),
            BASE_ASSET_TOKEN_ID,
            &[EsdtLocalRole::Burn],
        );
        b_mock.set_esdt_local_roles(
            unstake_sc_wrapper.address_ref(),
            LOCKED_TOKEN_ID,
            &[EsdtLocalRole::NftBurn],
        );

        // set user balance
        b_mock.set_esdt_balance(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            &rust_biguint!(USER_BALANCE),
        );
        b_mock.set_esdt_balance(
            &second_user,
            BASE_ASSET_TOKEN_ID,
            &rust_biguint!(USER_BALANCE),
        );

        Self {
            b_mock,
            owner,
            first_user,
            second_user,
            energy_factory_wrapper,
            unstake_sc_wrapper,
            fees_collector_mock: fees_collector_mock.address_ref().clone(),
        }
    }
}

impl<EnergyFactoryBuilder, UnstakeScBuilder>
    TokenUnstakeSetup<EnergyFactoryBuilder, UnstakeScBuilder>
where
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    UnstakeScBuilder: 'static + Copy + Fn() -> token_unstake::ContractObj<DebugApi>,
{
    pub fn lock(
        &mut self,
        caller: &Address,
        token_id: &[u8],
        amount: u64,
        lock_epochs: u64,
    ) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            caller,
            &self.energy_factory_wrapper,
            token_id,
            0,
            &rust_biguint!(amount),
            |sc| {
                sc.lock_tokens_endpoint(lock_epochs, OptionalValue::Some(managed_address!(caller)));
            },
        )
    }

    pub fn unlock(&mut self, caller: &Address, token_nonce: u64, amount: u64) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            caller,
            &self.energy_factory_wrapper,
            LOCKED_TOKEN_ID,
            token_nonce,
            &rust_biguint!(amount),
            |sc| {
                sc.unlock_tokens_endpoint();
            },
        )
    }

    pub fn unlock_early(&mut self, caller: &Address, token_nonce: u64, amount: u64) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            caller,
            &self.energy_factory_wrapper,
            LOCKED_TOKEN_ID,
            token_nonce,
            &rust_biguint!(amount),
            |sc| {
                sc.unlock_early();
            },
        )
    }

    pub fn reduce_lock_period(
        &mut self,
        caller: &Address,
        token_nonce: u64,
        amount: u64,
        new_lock_period: u64,
    ) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            caller,
            &self.energy_factory_wrapper,
            LOCKED_TOKEN_ID,
            token_nonce,
            &rust_biguint!(amount),
            |sc| {
                sc.reduce_lock_period(new_lock_period);
            },
        )
    }

    pub fn get_penalty_amount(
        &mut self,
        token_amount: u64,
        prev_lock_epochs: u64,
        new_lock_epochs: u64,
    ) -> num_bigint::BigUint {
        let mut result = rust_biguint!(0);
        self.b_mock
            .execute_query(&self.energy_factory_wrapper, |sc| {
                let managed_result = sc.calculate_penalty_amount(
                    &managed_biguint!(token_amount),
                    prev_lock_epochs,
                    new_lock_epochs,
                );
                result = to_rust_biguint(managed_result);
            })
            .assert_ok();

        result
    }

    pub fn get_user_energy(&mut self, user: &Address) -> num_bigint::BigUint {
        let mut result = rust_biguint!(0);
        self.b_mock
            .execute_query(&self.energy_factory_wrapper, |sc| {
                let managed_result = sc.get_energy_amount_for_user(managed_address!(user));
                result = to_rust_biguint(managed_result);
            })
            .assert_ok();

        result
    }
}

impl<EnergyFactoryBuilder, UnstakeScBuilder>
    TokenUnstakeSetup<EnergyFactoryBuilder, UnstakeScBuilder>
where
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    UnstakeScBuilder: 'static + Copy + Fn() -> token_unstake::ContractObj<DebugApi>,
{
    pub fn unbond(&mut self, user: &Address) -> TxResult {
        self.b_mock
            .execute_tx(user, &self.unstake_sc_wrapper, &rust_biguint!(0), |sc| {
                let _ = sc.claim_unlocked_tokens();
            })
    }

    pub fn cancel_unbond(&mut self, user: &Address) -> TxResult {
        self.b_mock
            .execute_tx(user, &self.unstake_sc_wrapper, &rust_biguint!(0), |sc| {
                let _ = sc.cancel_unbond();
            })
    }
}

pub fn to_rust_biguint(
    managed_biguint: multiversx_sc::types::BigUint<DebugApi>,
) -> num_bigint::BigUint {
    num_bigint::BigUint::from_bytes_be(managed_biguint.to_bytes_be().as_slice())
}

pub fn to_managed_biguint(
    rust_biguint: num_bigint::BigUint,
) -> multiversx_sc::types::BigUint<DebugApi> {
    multiversx_sc::types::BigUint::from_bytes_be(&rust_biguint.to_bytes_be())
}

pub fn to_start_of_month(unlock_epoch: u64) -> u64 {
    unlock_epoch - unlock_epoch % 30
}
