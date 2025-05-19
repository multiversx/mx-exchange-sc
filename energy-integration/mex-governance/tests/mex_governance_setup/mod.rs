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
    config::ConfigModule as _, incentive::IncentiveModule, vote::VoteModule, MEXGovernance,
};
use multiversx_sc::{
    imports::{MultiValue2, MultiValue3, StorageTokenWrapper},
    types::{Address, BigInt, ManagedAddress, MultiValueEncoded},
};
use multiversx_sc_scenario::{
    imports::TxResult,
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    whitebox_legacy::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};

use locking_module::lock_with_energy_module::LockWithEnergyModule;
use pair::{config::ConfigModule as _, Pair};
use pausable::{PausableModule, State};
use permissions_module::PermissionsModule as _;
use simple_lock::locked_token::LockedTokenModule;

pub const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-123456";
pub const MEX_TOKEN_ID: &[u8] = b"MEX-123456";
pub const USDC_TOKEN_ID: &[u8] = b"USDC-123456";
pub const HTM_TOKEN_ID: &[u8] = b"HTM-123456";
pub static LOCKED_TOKEN_ID: &[u8] = b"LOCKED-123456";
pub static LEGACY_LOCKED_TOKEN_ID: &[u8] = b"LEGACY-123456";
pub static WEGLDMEX_FARMING_TOKEN_ID: &[u8] = b"WMLPTOK-123456";
pub static WEGLDUSDC_FARMING_TOKEN_ID: &[u8] = b"WULPTOK-123456";
pub static WEGLDHTM_FARMING_TOKEN_ID: &[u8] = b"WHLPTOK-123456";
pub static WEGLDMEXFARM_TOKEN_ID: &[u8] = b"WMFARM-123456";
pub static WEGLDUSDCFARM_TOKEN_ID: &[u8] = b"WUFARM-123456";
pub static WEGLDHTMFARM_TOKEN_ID: &[u8] = b"WHFARM-123456";

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

pub struct GovSetup<PairBuilder, FarmLockedBuilder, EnergyFactoryBuilder, GovBuilder>
where
    PairBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    FarmLockedBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    GovBuilder: 'static + Copy + Fn() -> mex_governance::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub first_user: Address,
    pub second_user: Address,
    pub third_user: Address,
    pub pair_wm_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairBuilder>,
    pub pair_wu_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairBuilder>,
    pub pair_wh_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairBuilder>,
    pub farm_wm_wrapper:
        ContractObjWrapper<farm_with_locked_rewards::ContractObj<DebugApi>, FarmLockedBuilder>,
    pub farm_wu_wrapper:
        ContractObjWrapper<farm_with_locked_rewards::ContractObj<DebugApi>, FarmLockedBuilder>,
    pub farm_wh_wrapper:
        ContractObjWrapper<farm_with_locked_rewards::ContractObj<DebugApi>, FarmLockedBuilder>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory::ContractObj<DebugApi>, EnergyFactoryBuilder>,
    pub gov_wrapper: ContractObjWrapper<mex_governance::ContractObj<DebugApi>, GovBuilder>,
}

impl<PairBuilder, FarmLockedBuilder, EnergyFactoryBuilder, GovBuilder>
    GovSetup<PairBuilder, FarmLockedBuilder, EnergyFactoryBuilder, GovBuilder>
where
    PairBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    FarmLockedBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    GovBuilder: 'static + Copy + Fn() -> mex_governance::ContractObj<DebugApi>,
{
    pub fn new(
        pair_builder: PairBuilder,
        farm_builder: FarmLockedBuilder,
        energy_factory_builder: EnergyFactoryBuilder,
        gov_builder: GovBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let first_user = b_mock.create_user_account(&rust_zero);
        let second_user = b_mock.create_user_account(&rust_zero);
        let third_user = b_mock.create_user_account(&rust_zero);

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

        // init pairs
        // WEGLD-MEX pair
        let pair_wm_wrapper =
            b_mock.create_sc_account(&rust_biguint!(0), Some(&owner), pair_builder, "pair path");

        b_mock
            .execute_tx(&owner, &pair_wm_wrapper, &rust_zero, |sc| {
                let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
                let second_token_id = managed_token_id!(MEX_TOKEN_ID);
                let router_address = managed_address!(&owner);
                let router_owner_address = managed_address!(&owner);
                let total_fee_percent = 300u64;
                let special_fee_percent = 50u64;

                sc.init(
                    first_token_id,
                    second_token_id,
                    router_address,
                    router_owner_address,
                    total_fee_percent,
                    special_fee_percent,
                    ManagedAddress::<DebugApi>::zero(),
                    MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
                );

                let lp_token_id = managed_token_id!(WEGLDMEX_FARMING_TOKEN_ID);
                sc.lp_token_identifier().set(&lp_token_id);

                sc.state().set(State::Active);
            })
            .assert_ok();

        // WEGLD-USDC pair
        let pair_wu_wrapper =
            b_mock.create_sc_account(&rust_biguint!(0), Some(&owner), pair_builder, "pair path");

        b_mock
            .execute_tx(&owner, &pair_wu_wrapper, &rust_zero, |sc| {
                let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
                let second_token_id = managed_token_id!(USDC_TOKEN_ID);
                let router_address = managed_address!(&owner);
                let router_owner_address = managed_address!(&owner);
                let total_fee_percent = 300u64;
                let special_fee_percent = 50u64;

                sc.init(
                    first_token_id,
                    second_token_id,
                    router_address,
                    router_owner_address,
                    total_fee_percent,
                    special_fee_percent,
                    ManagedAddress::<DebugApi>::zero(),
                    MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
                );

                let lp_token_id = managed_token_id!(WEGLDUSDC_FARMING_TOKEN_ID);
                sc.lp_token_identifier().set(&lp_token_id);

                sc.state().set(State::Active);
            })
            .assert_ok();

        // WEGLD-HTM pair
        let pair_wh_wrapper =
            b_mock.create_sc_account(&rust_biguint!(0), Some(&owner), pair_builder, "pair path");

        b_mock
            .execute_tx(&owner, &pair_wh_wrapper, &rust_zero, |sc| {
                let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
                let second_token_id = managed_token_id!(HTM_TOKEN_ID);
                let router_address = managed_address!(&owner);
                let router_owner_address = managed_address!(&owner);
                let total_fee_percent = 300u64;
                let special_fee_percent = 50u64;

                sc.init(
                    first_token_id,
                    second_token_id,
                    router_address,
                    router_owner_address,
                    total_fee_percent,
                    special_fee_percent,
                    ManagedAddress::<DebugApi>::zero(),
                    MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
                );

                let lp_token_id = managed_token_id!(WEGLDHTM_FARMING_TOKEN_ID);
                sc.lp_token_identifier().set(&lp_token_id);

                sc.state().set(State::Active);
            })
            .assert_ok();

        // Init farm with locked rewards

        // Declare the governance SC
        let gov_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), gov_builder, "gov path");

        // WEGLD-MEX farm
        let farm_wm_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            farm_builder,
            "farm-with-locked-rewards.wasm",
        );

        b_mock
            .execute_tx(&owner, &farm_wm_wrapper, &rust_zero, |sc| {
                let reward_token_id = managed_token_id!(MEX_TOKEN_ID);
                let farming_token_id = managed_token_id!(WEGLDMEX_FARMING_TOKEN_ID);
                let division_safety_constant = managed_biguint!(DIV_SAFETY);
                let pair_address = managed_address!(&pair_wm_wrapper.address_ref());

                sc.init(
                    reward_token_id,
                    farming_token_id,
                    division_safety_constant,
                    pair_address,
                    managed_address!(&owner),
                    MultiValueEncoded::new(),
                );

                let farm_token_id = managed_token_id!(WEGLDMEXFARM_TOKEN_ID);
                sc.farm_token().set_token_id(farm_token_id);

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
                sc.set_locking_sc_address(managed_address!(energy_factory_wrapper.address_ref()));
                sc.set_lock_epochs(EPOCHS_IN_YEAR);
                sc.energy_factory_address()
                    .set(managed_address!(energy_factory_wrapper.address_ref()));

                sc.add_admin_endpoint(managed_address!(gov_wrapper.address_ref()));
            })
            .assert_ok();

        // WEGLD-USDC farm
        let farm_wu_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            farm_builder,
            "farm-with-locked-rewards.wasm",
        );

        b_mock
            .execute_tx(&owner, &farm_wu_wrapper, &rust_zero, |sc| {
                let reward_token_id = managed_token_id!(MEX_TOKEN_ID);
                let farming_token_id = managed_token_id!(WEGLDUSDC_FARMING_TOKEN_ID);
                let division_safety_constant = managed_biguint!(DIV_SAFETY);
                let pair_address = managed_address!(&pair_wu_wrapper.address_ref());

                sc.init(
                    reward_token_id,
                    farming_token_id,
                    division_safety_constant,
                    pair_address,
                    managed_address!(&owner),
                    MultiValueEncoded::new(),
                );

                let farm_token_id = managed_token_id!(WEGLDUSDCFARM_TOKEN_ID);
                sc.farm_token().set_token_id(farm_token_id);

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
                sc.set_locking_sc_address(managed_address!(energy_factory_wrapper.address_ref()));
                sc.set_lock_epochs(EPOCHS_IN_YEAR);
                sc.energy_factory_address()
                    .set(managed_address!(energy_factory_wrapper.address_ref()));

                sc.add_admin_endpoint(managed_address!(gov_wrapper.address_ref()));
            })
            .assert_ok();

        // WEGLD-HTM farm
        let farm_wh_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            farm_builder,
            "farm-with-locked-rewards.wasm",
        );

        b_mock
            .execute_tx(&owner, &farm_wh_wrapper, &rust_zero, |sc| {
                let reward_token_id = managed_token_id!(MEX_TOKEN_ID);
                let farming_token_id = managed_token_id!(WEGLDHTM_FARMING_TOKEN_ID);
                let division_safety_constant = managed_biguint!(DIV_SAFETY);
                let pair_address = managed_address!(&pair_wh_wrapper.address_ref());

                sc.init(
                    reward_token_id,
                    farming_token_id,
                    division_safety_constant,
                    pair_address,
                    managed_address!(&owner),
                    MultiValueEncoded::new(),
                );

                let farm_token_id = managed_token_id!(WEGLDHTMFARM_TOKEN_ID);
                sc.farm_token().set_token_id(farm_token_id);

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
                sc.set_locking_sc_address(managed_address!(energy_factory_wrapper.address_ref()));
                sc.set_lock_epochs(EPOCHS_IN_YEAR);
                sc.energy_factory_address()
                    .set(managed_address!(energy_factory_wrapper.address_ref()));

                sc.add_admin_endpoint(managed_address!(gov_wrapper.address_ref()));
            })
            .assert_ok();

        // Whitelist farms in energy factory
        b_mock
            .execute_tx(&owner, &energy_factory_wrapper, &rust_zero, |sc| {
                let mut farms = MultiValueEncoded::new();
                farms.push(managed_address!(farm_wm_wrapper.address_ref()));
                farms.push(managed_address!(farm_wu_wrapper.address_ref()));
                farms.push(managed_address!(farm_wh_wrapper.address_ref()));
                sc.add_to_unlocked_token_mint_whitelist(farms);
            })
            .assert_ok();

        // init governance sc
        b_mock
            .execute_tx(&owner, &gov_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_biguint!(DEFAULT_EMISSION_RATE),
                    managed_token_id!(MEX_TOKEN_ID),
                    managed_address!(energy_factory_wrapper.address_ref()),
                );
            })
            .assert_ok();

        Self {
            b_mock,
            owner,
            first_user,
            second_user,
            third_user,
            pair_wm_wrapper,
            pair_wu_wrapper,
            pair_wh_wrapper,
            farm_wm_wrapper,
            farm_wu_wrapper,
            farm_wh_wrapper,
            energy_factory_wrapper,
            gov_wrapper,
        }
    }

    pub fn vote(&mut self, user: Address, votes: Vec<MultiValue2<Address, u64>>) -> TxResult {
        self.b_mock
            .execute_tx(&user, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                let mut votes_managed = MultiValueEncoded::new();
                for vote in votes {
                    let (address, amount) = vote.into_tuple();
                    votes_managed
                        .push((managed_address!(&address), managed_biguint!(amount)).into());
                }

                let _ = sc.vote(votes_managed);
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
                    sc.user_energy(&managed_address!(&user)).set(&Energy::new(
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
}
