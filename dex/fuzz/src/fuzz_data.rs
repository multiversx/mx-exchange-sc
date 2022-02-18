#[cfg(test)]
pub mod fuzz_data_tests {
    elrond_wasm::imports!();
    elrond_wasm::derive_imports!();

    use std::cell::Cell;

    use ::config::ConfigModule;
    use elrond_wasm::types::{
        Address, BigUint, EsdtLocalRole, ManagedAddress, OptionalArg, TokenIdentifier,
    };

    use elrond_wasm_debug::managed_biguint;
    use elrond_wasm_debug::{
        managed_address, managed_token_id, rust_biguint, testing_framework::*, DebugApi,
    };

    type RustBigUint = num_bigint::BigUint;

    use farm::*;
    use pair::*;

    pub const FARM_WASM_PATH: &'static str = "farm/output/farm.wasm";
    pub const PAIR_WASM_PATH: &'static str = "pair/output/pair.wasm";

    pub const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
    pub const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef";
    pub const BUSD_TOKEN_ID: &[u8] = b"BUSD-abcdef";
    pub const WEME_LP_TOKEN_ID: &[u8] = b"WEMELP-abcdef";
    pub const WEBU_LP_TOKEN_ID: &[u8] = b"WEBULP-abcdef";
    pub const WEME_FARM_TOKEN_ID: &[u8] = b"WEMEFARM-abcdef";
    pub const WEBU_FARM_TOKEN_ID: &[u8] = b"WEBUFARM-abcdef";
    pub const MEX_FARM_TOKEN_ID: &[u8] = b"MEXFARM-abcdef";
    pub const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
    pub const MIN_FARMING_EPOCHS: u8 = 2;
    pub const PENALTY_PERCENT: u64 = 10;
    pub const USER_TOTAL_EGLD_TOKENS: u64 = 1_000_000;
    pub const USER_TOTAL_MEX_TOKENS: u64 = 100_000_000_000;
    pub const USER_TOTAL_WEGLD_TOKENS: u64 = 100_000_000_000;
    pub const USER_TOTAL_BUSD_TOKENS: u64 = 100_000_000_000;
    pub const TOTAL_FEE_PERCENT: u64 = 300;
    pub const SPECIAL_FEE_PERCENT: u64 = 50;

    #[derive(Clone, TopEncode)]
    pub struct FuzzDexExecutorInitArgs {
        pub num_users: u64,
        pub num_events: u64,
        pub remove_liquidity_prob: u64,
        pub add_liquidity_prob: u64,
        pub swap_prob: u64,
        pub query_pairs_prob: u64,
        pub enter_farm_prob: u64,
        pub exit_farm_prob: u64,
        pub claim_rewards_prob: u64,
        pub compound_rewards_prob: u64,
        pub increase_block_nonce_prob: u64,
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
                num_users: 1,
                num_events: 500,
                remove_liquidity_prob: 5,
                add_liquidity_prob: 20,
                swap_prob: 25,
                query_pairs_prob: 5,
                enter_farm_prob: 18,
                exit_farm_prob: 6,
                claim_rewards_prob: 20,
                compound_rewards_prob: 10,
                increase_block_nonce_prob: 100,
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
        pub owner_address: Address,
        pub fuzz_args: FuzzDexExecutorInitArgs,
        pub statistics: EventsStatistics,
        pub blockchain_wrapper: BlockchainStateWrapper,
        pub users: Vec<Address>,
        pub swap_pairs: Vec<PairSetup<PairObjBuilder>>,
        pub farms: Vec<FarmSetup<FarmObjBuilder>>,
    }

    impl<PairObjBuilder, FarmObjBuilder> FuzzerData<PairObjBuilder, FarmObjBuilder>
    where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        pub fn new(pair_builder: PairObjBuilder, farm_builder: FarmObjBuilder) -> Self {
            let egld_amount = rust_biguint!(USER_TOTAL_EGLD_TOKENS);

            let fuzz_args = FuzzDexExecutorInitArgs::new();
            let statistics = EventsStatistics::new();
            let mut blockchain_wrapper = BlockchainStateWrapper::new();
            let owner_addr = blockchain_wrapper.create_user_account(&egld_amount);

            let mut users = vec![];

            for _i in 1..=fuzz_args.num_users {
                let user = blockchain_wrapper.create_user_account(&egld_amount);
                blockchain_wrapper.set_esdt_balance(
                    &user,
                    WEGLD_TOKEN_ID,
                    &rust_biguint!(USER_TOTAL_WEGLD_TOKENS),
                );
                blockchain_wrapper.set_esdt_balance(
                    &user,
                    MEX_TOKEN_ID,
                    &rust_biguint!(USER_TOTAL_MEX_TOKENS),
                );
                blockchain_wrapper.set_esdt_balance(
                    &user,
                    BUSD_TOKEN_ID,
                    &rust_biguint!(USER_TOTAL_BUSD_TOKENS),
                );

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
                rust_biguint!(10000000000000000u64),
            );

            let second_farm = setup_farm(
                WEBU_FARM_TOKEN_ID,
                WEBU_LP_TOKEN_ID,
                MEX_TOKEN_ID,
                &owner_addr,
                &mut blockchain_wrapper,
                farm_builder,
                rust_biguint!(10000000000000000u64),
            );

            let third_farm = setup_farm(
                MEX_FARM_TOKEN_ID,
                MEX_TOKEN_ID,
                MEX_TOKEN_ID,
                &owner_addr,
                &mut blockchain_wrapper,
                farm_builder,
                rust_biguint!(10000000000000000u64),
            );

            let farms = vec![first_farm, second_farm, third_farm];

            FuzzerData {
                owner_address: owner_addr,
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
            Some(&owner_addr),
            pair_builder,
            PAIR_WASM_PATH,
        );

        blockchain_wrapper
            .execute_tx(&owner_addr, &pair_wrapper, &rust_zero, |sc| {
                let first_token_id = managed_token_id!(first_token);
                let second_token_id = managed_token_id!(second_token);
                let router_address = managed_address!(&owner_addr);
                let router_owner_address = managed_address!(&owner_addr);
                let total_fee_percent = TOTAL_FEE_PERCENT;
                let special_fee_percent = SPECIAL_FEE_PERCENT;

                sc.init(
                    first_token_id,
                    second_token_id,
                    router_address,
                    router_owner_address,
                    total_fee_percent,
                    special_fee_percent,
                    OptionalArg::None,
                );

                let lp_token_id = managed_token_id!(lp_token);
                config::ConfigModule::lp_token_identifier(&sc).set(&lp_token_id);

                config::ConfigModule::state(&sc).set(&config::State::Active);

                StateChange::Commit
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
            Some(&owner_addr),
            farm_builder,
            FARM_WASM_PATH,
        );

        // init farm contract

        blockchain_wrapper
            .execute_tx(&owner_addr, &farm_wrapper, &rust_zero, |sc| {
                let reward_token_id = managed_token_id!(reward_token);
                let farming_token_id = managed_token_id!(farming_token);
                let division_safety_constant = managed_biguint!(DIVISION_SAFETY_CONSTANT);
                let pair_address = managed_address!(&Address::zero());

                sc.init(
                    reward_token_id,
                    farming_token_id,
                    division_safety_constant,
                    pair_address,
                );

                let farm_token_id = managed_token_id!(farm_token);
                sc.farm_token_id().set(&farm_token_id);

                sc.per_block_reward_amount()
                    .set(&to_managed_biguint(per_block_reward_amount));
                sc.minimum_farming_epochs().set(&MIN_FARMING_EPOCHS);
                sc.penalty_percent().set(&PENALTY_PERCENT);

                sc.state().set(&::config::State::Active);
                sc.produce_rewards_enabled().set(&true);

                StateChange::Commit
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
            farm_wrapper,
        }
    }

    #[derive(Clone, PartialEq)]
    pub struct EventsStatistics {
        pub swap_fixed_input_hits: u64,
        pub swap_fixed_input_misses: u64,

        pub swap_fixed_output_hits: u64,
        pub swap_fixed_output_misses: u64,

        pub add_liquidity_hits: u64,
        pub add_liquidity_misses: u64,
        pub add_liquidity_price_checks: u64,

        pub remove_liquidity_hits: u64,
        pub remove_liquidity_misses: u64,
        pub remove_liquidity_price_checks: u64,

        pub query_pairs_hits: u64,
        pub query_pairs_misses: u64,

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

    impl EventsStatistics {
        pub fn new() -> EventsStatistics {
            EventsStatistics {
                swap_fixed_input_hits: 0,
                swap_fixed_input_misses: 0,
                swap_fixed_output_hits: 0,
                swap_fixed_output_misses: 0,
                add_liquidity_hits: 0,
                add_liquidity_misses: 0,
                add_liquidity_price_checks: 0,
                remove_liquidity_hits: 0,
                remove_liquidity_misses: 0,
                remove_liquidity_price_checks: 0,
                query_pairs_hits: 0,
                query_pairs_misses: 0,
                enter_farm_hits: 0,
                enter_farm_misses: 0,
                exit_farm_hits: 0,
                exit_farm_misses: 0,
                exit_farm_with_rewards: 0,
                claim_rewards_hits: 0,
                claim_rewards_misses: 0,
                claim_rewards_with_rewards: 0,
                compound_rewards_hits: 0,
                compound_rewards_misses: 0,
            }
        }
    }

    fn to_managed_biguint(value: RustBigUint) -> BigUint<DebugApi> {
        BigUint::from_bytes_be(&value.to_bytes_be())
    }
}
