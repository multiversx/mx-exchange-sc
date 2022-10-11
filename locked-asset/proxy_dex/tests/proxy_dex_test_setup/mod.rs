use config::ConfigModule;
use elrond_wasm::{
    contract_base::{CallableContract, ContractBase},
    elrond_codec::multi_types::OptionalValue,
    storage::mappers::StorageTokenWrapper,
    types::{Address, EsdtLocalRole, ManagedAddress, MultiValueEncoded},
};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};
use elrond_wasm_modules::pause::PauseModule;
use farm::Farm;
use farm_token::FarmTokenModule;
use pair::{config::ConfigModule as OtherConfigModule, safe_price::SafePriceModule, Pair};
use pausable::{PausableModule, State};
use proxy_dex::{proxy_common::ProxyCommonModule, sc_whitelist::ScWhitelistModule, ProxyDexImpl};
use simple_lock::locked_token::{LockedTokenAttributes, LockedTokenModule};
use simple_lock_energy::{lock_options::LockOptionsModule, SimpleLockEnergy};

// General
pub static MEX_TOKEN_ID: &[u8] = b"MEX-123456";
pub static WEGLD_TOKEN_ID: &[u8] = b"WEGLD-123456";
pub const EPOCHS_IN_YEAR: u64 = 365;
pub const USER_BALANCE: u64 = 1_000_000_000_000_000_000;

// Pair
pub static LP_TOKEN_ID: &[u8] = b"LPTOK-123456";

// Farm
pub static FARM_TOKEN_ID: &[u8] = b"FARM-123456";
pub const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000_000_000;
pub const PER_BLOCK_REWARD_AMOUNT: u64 = 5_000;

// Simple Lock
pub static LOCKED_TOKEN_ID: &[u8] = b"LOCKED-123456";
pub static LEGACY_LOCKED_TOKEN_ID: &[u8] = b"LEGACY-123456";
pub static LOCK_OPTIONS: &[u64] = &[EPOCHS_IN_YEAR, 5 * EPOCHS_IN_YEAR, 10 * EPOCHS_IN_YEAR]; // 1, 5 or 10 years
pub const MIN_PENALTY_PERCENTAGE: u16 = 1; // 0.01%
pub const MAX_PENALTY_PERCENTAGE: u16 = 10_000; // 100%
pub const FEES_BURN_PERCENTAGE: u16 = 10_000; // 100%

// Proxy
pub static WRAPPED_LP_TOKEN_ID: &[u8] = b"WPLP-123456";
pub static WRAPPED_FARM_TOKEN_ID: &[u8] = b"WPFARM-123456";

pub struct ProxySetup<ProxyObjBuilder, PairObjBuilder, FarmObjBuilder, SimpleLockObjBuilder>
where
    ProxyObjBuilder: 'static + Copy + Fn() -> proxy_dex::ContractObj<DebugApi>,
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    SimpleLockObjBuilder: 'static + Copy + Fn() -> simple_lock_energy::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub first_user: Address,
    pub second_user: Address,
    pub proxy_wrapper: ContractObjWrapper<proxy_dex::ContractObj<DebugApi>, ProxyObjBuilder>,
    pub pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
    pub farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
    pub simple_lock_wrapper:
        ContractObjWrapper<simple_lock_energy::ContractObj<DebugApi>, SimpleLockObjBuilder>,
}

impl<ProxyObjBuilder, PairObjBuilder, FarmObjBuilder, SimpleLockObjBuilder>
    ProxySetup<ProxyObjBuilder, PairObjBuilder, FarmObjBuilder, SimpleLockObjBuilder>
where
    ProxyObjBuilder: 'static + Copy + Fn() -> proxy_dex::ContractObj<DebugApi>,
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    SimpleLockObjBuilder: 'static + Copy + Fn() -> simple_lock_energy::ContractObj<DebugApi>,
{
    pub fn new(
        proxy_builder: ProxyObjBuilder,
        pair_builder: PairObjBuilder,
        farm_builder: FarmObjBuilder,
        simple_lock_builder: SimpleLockObjBuilder,
    ) -> Self {
        let _ = DebugApi::dummy();

        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let first_user = b_mock.create_user_account(&rust_zero);
        let second_user = b_mock.create_user_account(&rust_zero);

        b_mock.set_block_epoch(1);

        let pair_wrapper = setup_pair(&mut b_mock, &owner, pair_builder);
        let farm_wrapper = setup_farm(&mut b_mock, &owner, farm_builder);
        let simple_lock_wrapper = setup_simple_lock(&mut b_mock, &owner, simple_lock_builder);
        let proxy_wrapper = setup_proxy(
            &mut b_mock,
            &owner,
            proxy_builder,
            pair_wrapper.address_ref(),
            farm_wrapper.address_ref(),
            simple_lock_wrapper.address_ref(),
        );

        let user_balance = rust_biguint!(USER_BALANCE);
        b_mock.set_esdt_balance(&first_user, MEX_TOKEN_ID, &user_balance);
        b_mock.set_esdt_balance(&first_user, WEGLD_TOKEN_ID, &user_balance);

        b_mock.set_esdt_balance(&second_user, MEX_TOKEN_ID, &user_balance);
        b_mock.set_esdt_balance(&second_user, WEGLD_TOKEN_ID, &user_balance);

        // users lock tokens
        b_mock
            .execute_esdt_transfer(
                &first_user,
                &simple_lock_wrapper,
                MEX_TOKEN_ID,
                0,
                &user_balance,
                |sc| {
                    sc.lock_tokens_endpoint(LOCK_OPTIONS[0], OptionalValue::None);
                },
            )
            .assert_ok();

        b_mock
            .execute_esdt_transfer(
                &second_user,
                &simple_lock_wrapper,
                MEX_TOKEN_ID,
                0,
                &user_balance,
                |sc| {
                    sc.lock_tokens_endpoint(LOCK_OPTIONS[1], OptionalValue::None);
                },
            )
            .assert_ok();

        b_mock.check_nft_balance(
            &first_user,
            LOCKED_TOKEN_ID,
            1,
            &user_balance,
            Some(&LockedTokenAttributes::<DebugApi> {
                original_token_id: managed_token_id_wrapped!(MEX_TOKEN_ID),
                original_token_nonce: 0,
                unlock_epoch: 360,
            }),
        );

        b_mock.check_nft_balance(
            &second_user,
            LOCKED_TOKEN_ID,
            2,
            &user_balance,
            Some(&LockedTokenAttributes::<DebugApi> {
                original_token_id: managed_token_id_wrapped!(MEX_TOKEN_ID),
                original_token_nonce: 0,
                unlock_epoch: 1_800,
            }),
        );

        Self {
            b_mock,
            owner,
            first_user,
            second_user,
            proxy_wrapper,
            pair_wrapper,
            farm_wrapper,
            simple_lock_wrapper,
        }
    }
}

fn setup_pair<PairObjBuilder>(
    b_mock: &mut BlockchainStateWrapper,
    owner: &Address,
    pair_builder: PairObjBuilder,
) -> ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let pair_wrapper = b_mock.create_sc_account(&rust_zero, Some(&owner), pair_builder, "pair");

    b_mock
        .execute_tx(&owner, &pair_wrapper, &rust_zero, |sc| {
            let first_token_id = managed_token_id!(MEX_TOKEN_ID);
            let second_token_id = managed_token_id!(WEGLD_TOKEN_ID);
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

            let lp_token_id = managed_token_id!(LP_TOKEN_ID);
            sc.lp_token_identifier().set(&lp_token_id);

            sc.state().set(&State::Active);
            sc.set_max_observations_per_record(10);
        })
        .assert_ok();

    let lp_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
    b_mock.set_esdt_local_roles(pair_wrapper.address_ref(), LP_TOKEN_ID, &lp_token_roles[..]);

    pair_wrapper
}

fn setup_farm<FarmObjBuilder>(
    b_mock: &mut BlockchainStateWrapper,
    owner: &Address,
    farm_builder: FarmObjBuilder,
) -> ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let farm_wrapper = b_mock.create_sc_account(&rust_zero, Some(&owner), farm_builder, "farm");

    b_mock
        .execute_tx(owner, &farm_wrapper, &rust_zero, |sc| {
            let reward_token_id = managed_token_id!(MEX_TOKEN_ID);
            let farming_token_id = managed_token_id!(MEX_TOKEN_ID);
            let division_safety_constant = managed_biguint!(DIVISION_SAFETY_CONSTANT);
            let pair_address = managed_address!(&Address::zero());

            sc.init(
                reward_token_id,
                farming_token_id,
                division_safety_constant,
                pair_address,
                managed_address!(&owner),
                MultiValueEncoded::new(),
            );

            let farm_token_id = managed_token_id!(FARM_TOKEN_ID);
            sc.farm_token().set_token_id(&farm_token_id);

            sc.per_block_reward_amount()
                .set(&managed_biguint!(PER_BLOCK_REWARD_AMOUNT));

            sc.state().set(State::Active);
            sc.produce_rewards_enabled().set(true);
        })
        .assert_ok();

    let farm_token_roles = [
        EsdtLocalRole::NftCreate,
        EsdtLocalRole::NftAddQuantity,
        EsdtLocalRole::NftBurn,
    ];
    b_mock.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        FARM_TOKEN_ID,
        &farm_token_roles[..],
    );

    let farming_token_roles = [EsdtLocalRole::Burn];
    b_mock.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &farming_token_roles[..],
    );

    let reward_token_roles = [EsdtLocalRole::Mint];
    b_mock.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &reward_token_roles[..],
    );

    farm_wrapper
}

fn setup_simple_lock<SimpleLockObjBuilder>(
    b_mock: &mut BlockchainStateWrapper,
    owner: &Address,
    simple_lock_builder: SimpleLockObjBuilder,
) -> ContractObjWrapper<simple_lock_energy::ContractObj<DebugApi>, SimpleLockObjBuilder>
where
    SimpleLockObjBuilder: 'static + Copy + Fn() -> simple_lock_energy::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let simple_lock_wrapper = b_mock.create_sc_account(
        &rust_zero,
        Some(&owner),
        simple_lock_builder,
        "simple lock energy",
    );
    let dummy_sc_wrapper =
        b_mock.create_sc_account(&rust_zero, Some(&owner), DummySc::new, "dummy sc 1");

    b_mock
        .execute_tx(owner, &simple_lock_wrapper, &rust_zero, |sc| {
            let mut lock_options = MultiValueEncoded::new();
            for option in LOCK_OPTIONS {
                lock_options.push(*option);
            }

            sc.init(
                managed_token_id!(MEX_TOKEN_ID),
                managed_token_id!(LEGACY_LOCKED_TOKEN_ID),
                MIN_PENALTY_PERCENTAGE,
                MAX_PENALTY_PERCENTAGE,
                FEES_BURN_PERCENTAGE,
                managed_address!(dummy_sc_wrapper.address_ref()),
                managed_address!(dummy_sc_wrapper.address_ref()),
                lock_options,
            );

            assert_eq!(sc.max_lock_option().get(), *LOCK_OPTIONS.last().unwrap());

            sc.locked_token()
                .set_token_id(&managed_token_id!(LOCKED_TOKEN_ID));
            sc.set_paused(false);
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        simple_lock_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
    );
    b_mock.set_esdt_local_roles(
        simple_lock_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
            EsdtLocalRole::Transfer,
        ],
    );
    b_mock.set_esdt_local_roles(
        simple_lock_wrapper.address_ref(),
        LEGACY_LOCKED_TOKEN_ID,
        &[EsdtLocalRole::NftBurn],
    );

    simple_lock_wrapper
}

fn setup_proxy<ProxyObjBuilder>(
    b_mock: &mut BlockchainStateWrapper,
    owner: &Address,
    proxy_builder: ProxyObjBuilder,
    pair_addr: &Address,
    farm_addr: &Address,
    simple_lock_addr: &Address,
) -> ContractObjWrapper<proxy_dex::ContractObj<DebugApi>, ProxyObjBuilder>
where
    ProxyObjBuilder: 'static + Copy + Fn() -> proxy_dex::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let proxy_wrapper = b_mock.create_sc_account(&rust_zero, Some(&owner), proxy_builder, "proxy");

    b_mock
        .execute_tx(owner, &proxy_wrapper, &rust_zero, |sc| {
            let mut locked_token_factory_address_token_pairs = MultiValueEncoded::new();
            locked_token_factory_address_token_pairs.push(
                (
                    managed_token_id!(LOCKED_TOKEN_ID),
                    managed_address!(simple_lock_addr),
                )
                    .into(),
            );

            sc.init(
                managed_token_id!(MEX_TOKEN_ID),
                locked_token_factory_address_token_pairs,
            );

            sc.wrapped_lp_token()
                .set_token_id(&managed_token_id!(WRAPPED_LP_TOKEN_ID));
            sc.wrapped_farm_token()
                .set_token_id(&managed_token_id!(WRAPPED_FARM_TOKEN_ID));

            sc.intermediated_pairs().insert(managed_address!(pair_addr));
            sc.intermediated_farms().insert(managed_address!(farm_addr));
        })
        .assert_ok();

    b_mock.set_esdt_local_roles(
        proxy_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
    );

    b_mock.set_esdt_local_roles(
        proxy_wrapper.address_ref(),
        LOCKED_TOKEN_ID,
        &[EsdtLocalRole::NftBurn],
    );

    b_mock.set_esdt_local_roles(
        proxy_wrapper.address_ref(),
        WRAPPED_LP_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    b_mock.set_esdt_local_roles(
        proxy_wrapper.address_ref(),
        WRAPPED_FARM_TOKEN_ID,
        &[
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ],
    );

    proxy_wrapper
}

#[derive(Clone)]
pub struct DummySc {}

impl ContractBase for DummySc {
    type Api = DebugApi;
}

impl CallableContract for DummySc {
    fn call(&self, _fn_name: &[u8]) -> bool {
        true
    }

    fn clone_obj(&self) -> Box<dyn CallableContract> {
        Box::new(self.clone())
    }
}

impl DummySc {
    pub fn new() -> Self {
        DummySc {}
    }
}