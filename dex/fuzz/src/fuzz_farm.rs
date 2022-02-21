#[cfg(test)]
pub mod fuzz_farm_test {

    elrond_wasm::imports!();
    elrond_wasm::derive_imports!();

    use elrond_wasm::types::OptionalArg;
    use elrond_wasm_debug::tx_mock::TxInputESDT;
    use elrond_wasm_debug::{rust_biguint, testing_framework::*, DebugApi, HashMap};

    use crate::fuzz_data::fuzz_data_tests::*;
    use farm::*;

    use rand::{prelude::SliceRandom, Rng};

    pub fn enter_farm<PairObjBuilder, FarmObjBuilder>(
        fuzzer_data: &mut FuzzerData<PairObjBuilder, FarmObjBuilder>,
        farmer_info: &mut HashMap<Address, Vec<u64>>,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let farm_setup = fuzzer_data.farms.choose(&mut rand::thread_rng()).unwrap();
        let farm_nonce = farm_setup.farm_nonce.get();

        let lp_token_id = farm_setup.farming_token.as_bytes();

        let mut rng = rand::thread_rng();
        let seed = rng.gen_range(0..fuzzer_data.fuzz_args.enter_farm_max_value) + 1;

        let farm_in_amount = rust_biguint!(seed);

        let caller = fuzzer_data.users.choose(&mut rand::thread_rng()).unwrap();

        let lp_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller, lp_token_id, 0);

        if lp_token_before < farm_in_amount {
            println!("Not enough LP token user balance");
            fuzzer_data.statistics.enter_farm_misses += 1;

            return;
        }

        let mut payments = Vec::new();
        payments.push(TxInputESDT {
            token_identifier: lp_token_id.to_vec(),
            nonce: 0,
            value: farm_in_amount.clone(),
        });

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_multi_transfer(
            &caller,
            &farm_setup.farm_wrapper,
            &payments,
            |sc| {
                sc.enter_farm(OptionalArg::None);

                StateChange::Commit
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            fuzzer_data.statistics.enter_farm_hits += 1;
            farmer_info.entry(caller.clone()).or_default().push(farm_nonce);
            farm_setup.farm_nonce.set(farm_nonce + 1);
        } else {
            println!("Enter farm errors: {}", tx_result_string);
            fuzzer_data.statistics.enter_farm_misses += 1;
        }
    }

    pub fn exit_farm<PairObjBuilder, FarmObjBuilder>(
        fuzzer_data: &mut FuzzerData<PairObjBuilder, FarmObjBuilder>,
        farmer_info: &mut HashMap<Address, Vec<u64>>,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let farm_setup = fuzzer_data.farms.choose(&mut rand::thread_rng()).unwrap();

        let farm_token_id = farm_setup.farm_token.as_bytes();

        let mut rng = rand::thread_rng();
        let seed = rng.gen_range(0..fuzzer_data.fuzz_args.exit_farm_max_value) + 1;

        let farm_out_amount = rust_biguint!(seed);

        let caller = fuzzer_data.users.choose(&mut rand::thread_rng()).unwrap();

        let farm_token_nonce = match farmer_info.get(caller) {
            Some(s) => *s.choose(&mut rand::thread_rng()).unwrap(),
            None => 0,
        };

        let farm_token_before = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            &caller,
            farm_token_id,
            farm_token_nonce,
        );

        if farm_token_before < farm_out_amount {
            println!("Not enough farm token user balance");
            fuzzer_data.statistics.exit_farm_misses += 1;

            return;
        }

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_transfer(
            &caller,
            &farm_setup.farm_wrapper,
            farm_token_id,
            farm_token_nonce,
            &farm_out_amount,
            |sc| {
                sc.exit_farm(OptionalArg::None);

                StateChange::Commit
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            fuzzer_data.statistics.exit_farm_hits += 1;
        } else {
            println!("Exit farm error: {}", tx_result_string);
            fuzzer_data.statistics.exit_farm_misses += 1;
        }
    }

    pub fn claim_rewards<PairObjBuilder, FarmObjBuilder>(
        fuzzer_data: &mut FuzzerData<PairObjBuilder, FarmObjBuilder>,
        farmer_info: &mut HashMap<Address, Vec<u64>>,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let farm_setup = fuzzer_data.farms.choose(&mut rand::thread_rng()).unwrap();

        let farm_token_id = farm_setup.farm_token.as_bytes();

        let mut rng = rand::thread_rng();
        let seed = rng.gen_range(0..fuzzer_data.fuzz_args.claim_rewards_max_value) + 1;

        let farm_token_amount = rust_biguint!(seed);

        let caller = fuzzer_data.users.choose(&mut rand::thread_rng()).unwrap();

        let farm_token_nonce = match farmer_info.get(caller) {
            Some(s) => *s.choose(&mut rand::thread_rng()).unwrap(),
            None => 0,
        };

        let farm_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller, farm_token_id, farm_token_nonce);

        if farm_token_before < farm_token_amount {
            println!("Not enough farm token user balance");
            fuzzer_data.statistics.claim_rewards_misses += 1;

            return;
        }

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_transfer(
            &caller,
            &farm_setup.farm_wrapper,
            farm_token_id,
            farm_token_nonce,
            &farm_token_amount,
            |sc| {
                sc.claim_rewards(OptionalArg::None);

                StateChange::Commit
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            fuzzer_data.statistics.claim_rewards_hits += 1;
        } else {
            println!("Claim rewards error: {}", tx_result_string);
            fuzzer_data.statistics.claim_rewards_misses += 1;
        }
    }

    pub fn compound_rewards<PairObjBuilder, FarmObjBuilder>(
        fuzzer_data: &mut FuzzerData<PairObjBuilder, FarmObjBuilder>,
        farmer_info: &mut HashMap<Address, Vec<u64>>,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let farm_setup = fuzzer_data.farms.choose(&mut rand::thread_rng()).unwrap();

        let farm_token_id = farm_setup.farm_token.as_bytes();
        let farming_token_id = farm_setup.farming_token.as_bytes();
        let reward_token_id = farm_setup.reward_token.as_bytes();

        if farming_token_id != reward_token_id {
            println!("Farming token id is different from reward token id");
            fuzzer_data.statistics.compound_rewards_misses += 1;
            return;
        }

        let mut rng = rand::thread_rng();
        let seed = rng.gen_range(0..fuzzer_data.fuzz_args.compound_rewards_max_value) + 1;

        let farm_token_amount = rust_biguint!(seed);

        let caller = fuzzer_data.users.choose(&mut rand::thread_rng()).unwrap();

        let farm_token_nonce = match farmer_info.get(caller) {
            Some(s) => *s.choose(&mut rand::thread_rng()).unwrap(),
            None => 0,
        };

        let farm_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller, farm_token_id, farm_token_nonce);

        if farm_token_before < farm_token_amount {
            println!("Not enough farm token user balance");
            fuzzer_data.statistics.compound_rewards_misses += 1;

            return;
        }

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_transfer(
            &caller,
            &farm_setup.farm_wrapper,
            farm_token_id,
            farm_token_nonce,
            &farm_token_amount,
            |sc| {
                sc.compound_rewards(OptionalArg::None);

                StateChange::Commit
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            fuzzer_data.statistics.compound_rewards_hits += 1;
        } else {
            println!("Compound rewards error: {}", tx_result_string);
            fuzzer_data.statistics.compound_rewards_misses += 1;
        }
    }
}
