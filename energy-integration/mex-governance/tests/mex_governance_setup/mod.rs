#![allow(deprecated)]

use config::ConfigModule;
use energy_factory::{
    energy::EnergyModule, unlocked_token_transfer::UnlockedTokenTransferModule, SimpleLockEnergy,
};
use energy_query::{Energy, EnergyQueryModule};
use farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule;
use farm_token::FarmTokenModule;
use farm_with_locked_rewards::Farm;

use mex_governance::{
    config::ConfigModule as _, external_interactions::farm_interactions::FarmInteractionsModule,
    incentive::IncentiveModule, vote::VoteModule, MEXGovernance,
};
use multiversx_sc::{
    imports::{MultiValue2, MultiValue3, StorageTokenWrapper},
    types::{Address, BigInt, MultiValueEncoded},
};
use multiversx_sc_scenario::{
    imports::TxResult,
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    whitebox_legacy::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};

use locking_module::lock_with_energy_module::LockWithEnergyModule;
use pausable::{PausableModule, State};
use permissions_module::PermissionsModule as _;
use simple_lock::locked_token::LockedTokenModule;

pub const MEX_TOKEN_ID: &[u8] = b"MEX-123456";
pub static LOCKED_TOKEN_ID: &[u8] = b"LOCKED-123456";
pub static LEGACY_LOCKED_TOKEN_ID: &[u8] = b"LEGACY-123456";
pub static FARMING_TOKEN_ID: &[u8] = b"LPTOK-123456";
pub static FARM_TOKEN_ID: &[u8] = b"FARMTOK-123456";

pub const MAX_REWARDS_FACTOR: u64 = 10;
pub const USER_REWARDS_ENERGY_CONST: u64 = 3;
pub const USER_REWARDS_FARM_CONST: u64 = 2;
pub const MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;
pub const MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;
pub const DEFAULT_EMISSION_RATE: u64 = 10_000;

const DIV_SAFETY: u64 = 1_000_000_000_000;
pub const EPOCHS_IN_YEAR: u64 = 360;
pub const PER_BLOCK_REWARD_AMOUNT: u64 = 1_000;

pub static LOCK_OPTIONS: &[u64] = &[EPOCHS_IN_YEAR, 2 * EPOCHS_IN_YEAR, 4 * EPOCHS_IN_YEAR];
pub static PENALTY_PERCENTAGES: &[u64] = &[4_000, 6_000, 8_000];

pub struct GovSetup<FarmLockedBuilder, EnergyFactoryBuilder, GovBuilder>
where
    FarmLockedBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    GovBuilder: 'static + Copy + Fn() -> mex_governance::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub first_user: Address,
    pub second_user: Address,
    pub third_user: Address,
    pub farms: Vec<Address>,
    pub farm_wrappers:
        Vec<ContractObjWrapper<farm_with_locked_rewards::ContractObj<DebugApi>, FarmLockedBuilder>>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory::ContractObj<DebugApi>, EnergyFactoryBuilder>,
    pub gov_wrapper: ContractObjWrapper<mex_governance::ContractObj<DebugApi>, GovBuilder>,
    pub dummy_pair_address: Address,
}

impl<FarmLockedBuilder, EnergyFactoryBuilder, GovBuilder>
    GovSetup<FarmLockedBuilder, EnergyFactoryBuilder, GovBuilder>
where
    FarmLockedBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    GovBuilder: 'static + Copy + Fn() -> mex_governance::ContractObj<DebugApi>,
{
    pub fn new(
        farm_builder: FarmLockedBuilder,
        energy_factory_builder: EnergyFactoryBuilder,
        gov_builder: GovBuilder,
    ) -> Self {
        Self::new_with_farms(farm_builder, energy_factory_builder, gov_builder, 3)
    }

    pub fn new_with_farms(
        farm_builder: FarmLockedBuilder,
        energy_factory_builder: EnergyFactoryBuilder,
        gov_builder: GovBuilder,
        initial_farm_count: usize,
    ) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let first_user = b_mock.create_user_account(&rust_zero);
        let second_user = b_mock.create_user_account(&rust_zero);
        let third_user = b_mock.create_user_account(&rust_zero);

        // Create dummy pair address
        let dummy_pair_address = b_mock.create_user_account(&rust_zero);

        // init energy factory
        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            energy_factory_builder,
            "energy factory path",
        );

        b_mock
            .execute_tx(&owner, &energy_factory_wrapper, &rust_zero, |sc| {
                let mut lock_options = MultiValueEncoded::new();
                for (option, penalty) in LOCK_OPTIONS.iter().zip(PENALTY_PERCENTAGES.iter()) {
                    lock_options.push((*option, *penalty).into());
                }

                sc.init(
                    managed_token_id!(MEX_TOKEN_ID),
                    managed_token_id!(LEGACY_LOCKED_TOKEN_ID),
                    managed_address!(energy_factory_wrapper.address_ref()),
                    0,
                    lock_options,
                );

                sc.locked_token()
                    .set_token_id(managed_token_id!(LOCKED_TOKEN_ID));
            })
            .assert_ok();

        // init governance sc
        let gov_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), gov_builder, "gov path");

        b_mock
            .execute_tx(&owner, &gov_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_biguint!(DEFAULT_EMISSION_RATE),
                    managed_token_id!(MEX_TOKEN_ID),
                    managed_address!(energy_factory_wrapper.address_ref()),
                );
            })
            .assert_ok();

        let mut setup = Self {
            b_mock,
            owner,
            first_user,
            second_user,
            third_user,
            farms: Vec::new(),
            farm_wrappers: Vec::new(),
            energy_factory_wrapper,
            gov_wrapper,
            dummy_pair_address,
        };

        // Deploy initial farms
        if initial_farm_count > 0 {
            setup.deploy_and_whitelist_farms(farm_builder, initial_farm_count);
        }

        setup
    }

    pub fn deploy_and_whitelist_farms(&mut self, farm_builder: FarmLockedBuilder, count: usize) {
        let mut new_farm_addresses = Vec::new();

        for _i in 0..count {
            // Deploy farm
            let farm_wrapper = self.b_mock.create_sc_account(
                &rust_biguint!(0),
                Some(&self.owner),
                farm_builder,
                "farm path",
            );

            let farm_address = farm_wrapper.address_ref().clone();
            new_farm_addresses.push(farm_address.clone());

            // Initialize farm
            self.b_mock
                .execute_tx(&self.owner, &farm_wrapper, &rust_biguint!(0), |sc| {
                    let reward_token_id = managed_token_id!(MEX_TOKEN_ID);
                    let farming_token_id = managed_token_id!(FARMING_TOKEN_ID);
                    let division_safety_constant = managed_biguint!(DIV_SAFETY);
                    let pair_address = managed_address!(&self.dummy_pair_address);

                    sc.init(
                        reward_token_id,
                        farming_token_id,
                        division_safety_constant,
                        pair_address,
                        managed_address!(&self.owner),
                        MultiValueEncoded::new(),
                    );

                    let farm_token = managed_token_id!(FARM_TOKEN_ID);
                    sc.farm_token().set_token_id(farm_token);

                    sc.per_block_reward_amount()
                        .set(&managed_biguint!(PER_BLOCK_REWARD_AMOUNT));

                    sc.state().set(State::Active);
                    sc.produce_rewards_enabled().set(true);

                    sc.set_boosted_yields_factors(
                        managed_biguint!(MAX_REWARDS_FACTOR),
                        managed_biguint!(USER_REWARDS_ENERGY_CONST),
                        managed_biguint!(USER_REWARDS_FARM_CONST),
                        managed_biguint!(MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS),
                        managed_biguint!(MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS),
                    );
                    sc.set_locking_sc_address(managed_address!(self
                        .energy_factory_wrapper
                        .address_ref()));
                    sc.set_lock_epochs(EPOCHS_IN_YEAR);
                    sc.energy_factory_address()
                        .set(managed_address!(self.energy_factory_wrapper.address_ref()));

                    sc.add_admin_endpoint(managed_address!(self.gov_wrapper.address_ref()));
                })
                .assert_ok();

            // Store farm info
            self.farms.push(farm_address);
            self.farm_wrappers.push(farm_wrapper);
        }

        // Whitelist all new farms in energy factory
        if !new_farm_addresses.is_empty() {
            self.b_mock
                .execute_tx(
                    &self.owner,
                    &self.energy_factory_wrapper,
                    &rust_biguint!(0),
                    |sc| {
                        let mut farms = MultiValueEncoded::new();
                        for farm_addr in &new_farm_addresses {
                            farms.push(managed_address!(farm_addr));
                        }
                        sc.add_to_unlocked_token_mint_whitelist(farms);
                    },
                )
                .assert_ok();
        }
    }

    // Helper methods for backward compatibility
    pub fn get_farm_address(&self, index: usize) -> Address {
        self.farms
            .get(index)
            .expect("Farm index out of bounds")
            .clone()
    }

    // Keep existing methods for compatibility
    pub fn vote(&mut self, user: Address, votes: Vec<MultiValue2<Address, u64>>) -> TxResult {
        self.b_mock
            .execute_tx(&user, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                let mut votes_managed = MultiValueEncoded::new();
                for vote in votes {
                    let (address, amount) = vote.into_tuple();
                    votes_managed
                        .push((managed_address!(&address), managed_biguint!(amount)).into());
                }

                sc.vote(votes_managed);
            })
    }

    pub fn incentivize_farm(
        &mut self,
        farm_address: Address,
        user: Address,
        amount: u64,
        week: usize,
    ) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            &user,
            &self.gov_wrapper,
            MEX_TOKEN_ID,
            0,
            &rust_biguint!(amount),
            |sc| {
                let mut incentives = MultiValueEncoded::new();
                incentives.push(MultiValue3((
                    managed_address!(&farm_address),
                    managed_biguint!(amount),
                    week,
                )));
                sc.incentivize_farm(incentives);
            },
        )
    }

    pub fn claim_incentives(&mut self, user: Address, week: usize) -> TxResult {
        self.b_mock
            .execute_tx(&user, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.claim_incentive(week);
            })
    }

    pub fn set_user_energy(
        &mut self,
        user: Address,
        energy: u64,
        last_update_epoch: u64,
        locked_tokens: u64,
    ) {
        self.b_mock
            .execute_tx(
                &self.owner,
                &self.energy_factory_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.user_energy(&managed_address!(&user)).set(Energy::new(
                        BigInt::from(managed_biguint!(energy)),
                        last_update_epoch,
                        managed_biguint!(locked_tokens),
                    ));
                },
            )
            .assert_ok();
    }

    pub fn blacklist_farm(&mut self, farm_address: Address) -> TxResult {
        self.b_mock
            .execute_tx(&self.owner, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                let mut farms = MultiValueEncoded::new();
                farms.push(managed_address!(&farm_address));
                sc.blacklist_farm(farms);
            })
    }

    pub fn set_farm_emissions(&mut self) -> TxResult {
        self.b_mock
            .execute_tx(&self.owner, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.set_farm_emissions();
            })
    }
}
