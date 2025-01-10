#[cfg(test)]
pub mod fuzz_data_tests {
    #![allow(deprecated)]

    multiversx_sc::imports!();
    multiversx_sc::derive_imports!();

    use ::config::ConfigModule;
    use farm::exit_penalty::ExitPenaltyModule;
    use farm::*;
    use farm_token::FarmTokenModule;
    use multiversx_sc::types::{Address, BigUint, EsdtLocalRole};
    use multiversx_sc_scenario::{
        managed_address, managed_biguint, managed_token_id, rust_biguint, whitebox_legacy::*,
        DebugApi,
    };
    use pair::*;
    use pausable::{PausableModule, State};
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use std::cell::Cell;
    use std::collections::HashMap;

    type RustBigUint = num_bigint::BigUint;

    pub const FARM_WASM_PATH: &str = "farm/output/farm.wasm";
    pub const PAIR_WASM_PATH: &str = "pair/output/pair.wasm";

    pub const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
    pub const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef";
    pub const BUSD_TOKEN_ID: &[u8] = b"BUSD-abcdef";
    pub const WEME_LP_TOKEN_ID: &[u8] = b"WEMELP-abcdef";
    pub const WEBU_LP_TOKEN_ID: &[u8] = b"WEBULP-abcdef";
    pub const WEME_FARM_TOKEN_ID: &[u8] = b"WEMEFARM-abcdef";
    pub const WEBU_FARM_TOKEN_ID: &[u8] = b"WEBUFARM-abcdef";
    pub const MEX_FARM_TOKEN_ID: &[u8] = b"MEXFARM-abcdef";
    pub const MIN_FARMING_EPOCHS: u64 = 2;
    pub const FARM_PENALTY_PERCENT: u64 = 10;
    pub const OWNER_EGLD_BALANCE: u64 = 100_000_000;
    pub const USER_TOTAL_MEX_TOKENS: u64 = 100_000_000_000;
    pub const USER_TOTAL_WEGLD_TOKENS: u64 = 100_000_000_000;
    pub const USER_TOTAL_BUSD_TOKENS: u64 = 100_000_000_000;
    pub const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;

    // Price discovery constants
    pub const TOTAL_FEE_PERCENT: u64 = 300;
    pub const SPECIAL_FEE_PERCENT: u64 = 50;

    #[derive(Clone, TopEncode)]
    pub struct FuzzDexExecutorInitArgs {
        pub num_users: u64,
        pub num_events: u64,
        pub remove_liquidity_prob: u64,
        pub add_liquidity_prob: u64,
        pub swap_prob: u64,
        pub enter_farm_prob: u64,
        pub exit_farm_prob: u64,
        pub claim_rewards_prob: u64,
        pub compound_rewards_prob: u64,
        pub block_nonce_increase: u64,
        pub compound_rewards_max_value: u64,
        pub token_deposit_max_value: u64,
        pub remove_liquidity_max_value: u64,
        pub add_liquidity_max_value: u64,
        pub swap_max_value: u64,
        pub enter_farm_max_value: u64,
        pub exit_farm_max_value: u64,
        pub claim_rewards_max_value: u64,
    }

    impl FuzzDexExecutorInitArgs {
        pub fn new() -> Self {
            FuzzDexExecutorInitArgs {
                num_users: 5,
                num_events: 500,
                remove_liquidity_prob: 10,
                add_liquidity_prob: 20,
                swap_prob: 25,
                enter_farm_prob: 20,
                exit_farm_prob: 10,
                claim_rewards_prob: 15,
                compound_rewards_prob: 10,
                block_nonce_increase: 1,
                compound_rewards_max_value: 1000000u64,
                token_deposit_max_value: 50000000u64,
                remove_liquidity_max_value: 1000000000u64,
                add_liquidity_max_value: 1000000000u64,
                swap_max_value: 10000000u64,
                enter_farm_max_value: 100000000u64,
                exit_farm_max_value: 1000000u64,
                claim_rewards_max_value: 1000000u64,
            }
        }
    }

    pub struct FuzzerData<PairObjBuilder, FarmObjBuilder>
    where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        pub rng: StdRng,
        pub fuzz_args: FuzzDexExecutorInitArgs,
        pub statistics: EventsStatistics,
        pub blockchain_wrapper: BlockchainStateWrapper,
        pub users: Vec<User>,
        pub swap_pairs: Vec<PairSetup<PairObjBuilder>>,
        pub farms: Vec<FarmSetup<FarmObjBuilder>>,
    }

    impl<PairObjBuilder, FarmObjBuilder> FuzzerData<PairObjBuilder, FarmObjBuilder>
    where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        pub fn new(seed: u64, pair_builder: PairObjBuilder, farm_builder: FarmObjBuilder) -> Self {
            let egld_amount = rust_biguint!(OWNER_EGLD_BALANCE);

            let rng = StdRng::seed_from_u64(seed);
            let fuzz_args = FuzzDexExecutorInitArgs::new();
            let statistics = EventsStatistics::default();
            let mut blockchain_wrapper = BlockchainStateWrapper::new();
            let owner_addr = blockchain_wrapper.create_user_account(&egld_amount);

            let mut users = vec![];

            for _ in 1..=fuzz_args.num_users {
                let user_address = blockchain_wrapper.create_user_account(&egld_amount);
                blockchain_wrapper.set_esdt_balance(
                    &user_address,
                    WEGLD_TOKEN_ID,
                    &rust_biguint!(USER_TOTAL_WEGLD_TOKENS),
                );
                blockchain_wrapper.set_esdt_balance(
                    &user_address,
                    MEX_TOKEN_ID,
                    &rust_biguint!(USER_TOTAL_MEX_TOKENS),
                );
                blockchain_wrapper.set_esdt_balance(
                    &user_address,
                    BUSD_TOKEN_ID,
                    &rust_biguint!(USER_TOTAL_BUSD_TOKENS),
                );

                let user = User {
                    address: user_address,
                };

                users.push(user);
            }

            let first_swap_pair = setup_pair(
                WEGLD_TOKEN_ID,
                MEX_TOKEN_ID,
                WEME_LP_TOKEN_ID,
                &owner_addr,
                &mut blockchain_wrapper,
                pair_builder,
            );

            let second_swap_pair = setup_pair(
                WEGLD_TOKEN_ID,
                BUSD_TOKEN_ID,
                WEBU_LP_TOKEN_ID,
                &owner_addr,
                &mut blockchain_wrapper,
                pair_builder,
            );

            let swap_pairs = vec![first_swap_pair, second_swap_pair];

            let first_farm = setup_farm(
                WEME_FARM_TOKEN_ID,
                WEME_LP_TOKEN_ID,
                MEX_TOKEN_ID,
                &owner_addr,
                &mut blockchain_wrapper,
                farm_builder,
                rust_biguint!(1000000u64),
            );

            let second_farm = setup_farm(
                WEBU_FARM_TOKEN_ID,
                WEBU_LP_TOKEN_ID,
                MEX_TOKEN_ID,
                &owner_addr,
                &mut blockchain_wrapper,
                farm_builder,
                rust_biguint!(1000000u64),
            );

            let third_farm = setup_farm(
                MEX_FARM_TOKEN_ID,
                MEX_TOKEN_ID,
                MEX_TOKEN_ID,
                &owner_addr,
                &mut blockchain_wrapper,
                farm_builder,
                rust_biguint!(1000000u64),
            );

            let farms = vec![first_farm, second_farm, third_farm];

            FuzzerData {
                rng,
                fuzz_args,
                statistics,
                blockchain_wrapper,
                users,
                swap_pairs,
                farms,
            }
        }
    }

    #[derive()]
    pub struct User {
        pub address: Address,
    }

    #[derive()]
    #[allow(dead_code)]
    pub struct PairSetup<PairObjBuilder>
    where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    {
        pub first_token: String,
        pub second_token: String,
        pub lp_token: String,
        pub pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
    }

    pub fn setup_pair<PairObjBuilder>(
        first_token: &[u8],
        second_token: &[u8],
        lp_token: &[u8],
        owner_addr: &Address,
        blockchain_wrapper: &mut BlockchainStateWrapper,
        pair_builder: PairObjBuilder,
    ) -> PairSetup<PairObjBuilder>
    where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    {
        let rust_zero = rust_biguint!(0u64);

        let pair_wrapper = blockchain_wrapper.create_sc_account(
            &rust_zero,
            Some(owner_addr),
            pair_builder,
            PAIR_WASM_PATH,
        );

        blockchain_wrapper
            .execute_tx(owner_addr, &pair_wrapper, &rust_zero, |sc| {
                let first_token_id = managed_token_id!(first_token);
                let second_token_id = managed_token_id!(second_token);
                let router_address = managed_address!(owner_addr);
                let router_owner_address = managed_address!(owner_addr);
                let total_fee_percent = TOTAL_FEE_PERCENT;
                let special_fee_percent = SPECIAL_FEE_PERCENT;

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

                let lp_token_id = managed_token_id!(lp_token);
                config::ConfigModule::lp_token_identifier(&sc).set(&lp_token_id);

                pausable::PausableModule::state(&sc).set(pausable::State::Active);
            })
            .assert_ok();

        let lp_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
        blockchain_wrapper.set_esdt_local_roles(
            pair_wrapper.address_ref(),
            lp_token,
            &lp_token_roles[..],
        );

        let first_token_string = String::from_utf8(first_token.to_vec()).unwrap();
        let second_token_string = String::from_utf8(second_token.to_vec()).unwrap();
        let lp_token_string = String::from_utf8(lp_token.to_vec()).unwrap();

        PairSetup {
            first_token: first_token_string,
            second_token: second_token_string,
            lp_token: lp_token_string,
            pair_wrapper,
        }
    }

    #[allow(dead_code)]
    pub struct FarmSetup<FarmObjBuilder>
    where
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        pub farm_token: String,
        pub farming_token: String,
        pub reward_token: String,
        pub farm_nonce: Cell<u64>,
        pub farmer_info: HashMap<Address, Vec<u64>>,
        pub farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
    }

    pub fn setup_farm<FarmObjBuilder>(
        farm_token: &[u8],
        farming_token: &[u8],
        reward_token: &[u8],
        owner_addr: &Address,
        blockchain_wrapper: &mut BlockchainStateWrapper,
        farm_builder: FarmObjBuilder,
        per_block_reward_amount: RustBigUint,
    ) -> FarmSetup<FarmObjBuilder>
    where
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let rust_zero = rust_biguint!(0u64);

        let farm_wrapper = blockchain_wrapper.create_sc_account(
            &rust_zero,
            Some(owner_addr),
            farm_builder,
            FARM_WASM_PATH,
        );

        blockchain_wrapper
            .execute_tx(owner_addr, &farm_wrapper, &rust_zero, |sc| {
                let reward_token_id = managed_token_id!(reward_token);
                let farming_token_id = managed_token_id!(farming_token);
                let division_safety_constant = managed_biguint!(DIVISION_SAFETY_CONSTANT);
                let pair_address = managed_address!(&Address::zero());

                sc.init(
                    reward_token_id,
                    farming_token_id,
                    division_safety_constant,
                    pair_address,
                    ManagedAddress::<DebugApi>::zero(),
                    MultiValueEncoded::new(),
                );

                let farm_token_id = managed_token_id!(farm_token);
                sc.farm_token().set_token_id(farm_token_id);

                sc.per_block_reward_amount()
                    .set(&to_managed_biguint(per_block_reward_amount));
                sc.minimum_farming_epochs().set(MIN_FARMING_EPOCHS);
                sc.penalty_percent().set(FARM_PENALTY_PERCENT);

                sc.state().set(State::Active);
                sc.produce_rewards_enabled().set(true);
            })
            .assert_ok();

        let farm_token_roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ];
        blockchain_wrapper.set_esdt_local_roles(
            farm_wrapper.address_ref(),
            farm_token,
            &farm_token_roles[..],
        );

        let farming_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
        blockchain_wrapper.set_esdt_local_roles(
            farm_wrapper.address_ref(),
            farming_token,
            &farming_token_roles[..],
        );

        let reward_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
        blockchain_wrapper.set_esdt_local_roles(
            farm_wrapper.address_ref(),
            reward_token,
            &reward_token_roles[..],
        );

        let farm_token_string = String::from_utf8(farm_token.to_vec()).unwrap();
        let farming_token_string = String::from_utf8(farming_token.to_vec()).unwrap();
        let reward_token_string = String::from_utf8(reward_token.to_vec()).unwrap();

        FarmSetup {
            farm_token: farm_token_string,
            farming_token: farming_token_string,
            reward_token: reward_token_string,
            farm_nonce: Cell::new(1),
            farmer_info: HashMap::new(),
            farm_wrapper,
        }
    }

    #[derive(Clone, PartialEq, Default)]
    pub struct EventsStatistics {
        pub swap_fixed_input_hits: u64,
        pub swap_fixed_input_misses: u64,

        pub swap_fixed_output_hits: u64,
        pub swap_fixed_output_misses: u64,

        pub add_liquidity_hits: u64,
        pub add_liquidity_misses: u64,

        pub remove_liquidity_hits: u64,
        pub remove_liquidity_misses: u64,

        pub enter_farm_hits: u64,
        pub enter_farm_misses: u64,

        pub exit_farm_hits: u64,
        pub exit_farm_misses: u64,
        pub exit_farm_with_rewards: u64,

        pub claim_rewards_hits: u64,
        pub claim_rewards_misses: u64,
        pub claim_rewards_with_rewards: u64,

        pub compound_rewards_hits: u64,
        pub compound_rewards_misses: u64,
    }

    pub fn to_managed_biguint(value: RustBigUint) -> BigUint<DebugApi> {
        BigUint::from_bytes_be(&value.to_bytes_be())
    }
}
