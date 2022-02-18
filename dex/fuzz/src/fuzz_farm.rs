#[cfg(test)]
pub mod fuzz_farm_test {

    elrond_wasm::imports!();
    elrond_wasm::derive_imports!();

    use elrond_wasm::types::OptionalArg;
    use elrond_wasm_debug::tx_mock::TxInputESDT;
    use elrond_wasm_debug::{rust_biguint, testing_framework::*, DebugApi};

    use crate::fuzz_data::fuzz_data_tests::*;
    use farm::*;

    use rand::{prelude::SliceRandom, Rng};

    pub fn enter_farm<PairObjBuilder, FarmObjBuilder>(
        fuzzer_data: &mut FuzzerData<PairObjBuilder, FarmObjBuilder>,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let farm_setup = fuzzer_data.farms.choose(&mut rand::thread_rng()).unwrap();

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
            let statistic_value = fuzzer_data.statistics.enter_farm_misses.get() + 1;
            fuzzer_data
                .statistics
                .enter_farm_misses
                .set(statistic_value);

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
            let statistic_value = fuzzer_data.statistics.enter_farm_hits.get() + 1;
            fuzzer_data.statistics.enter_farm_hits.set(statistic_value);
        } else {
            println!("Enter farm errors: {}", tx_result_string);
            let statistic_value = fuzzer_data.statistics.enter_farm_misses.get() + 1;
            fuzzer_data
                .statistics
                .enter_farm_misses
                .set(statistic_value);
        }
    }

    pub fn exit_farm<PairObjBuilder, FarmObjBuilder>(
        fuzzer_data: &mut FuzzerData<PairObjBuilder, FarmObjBuilder>,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let farm_token_nonce = 1u64;

        let farm_setup = fuzzer_data.farms.choose(&mut rand::thread_rng()).unwrap();

        let farm_token_id = farm_setup.farm_token.as_bytes();

        let mut rng = rand::thread_rng();
        let seed = rng.gen_range(0..fuzzer_data.fuzz_args.exit_farm_max_value) + 1;

        let farm_out_amount = rust_biguint!(seed);

        let caller = fuzzer_data.users.choose(&mut rand::thread_rng()).unwrap();

        let farm_token_before = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            &caller,
            farm_token_id,
            farm_token_nonce,
        );

        if farm_token_before < farm_out_amount {
            println!("Not enough farm token user balance");
            let statistic_value = fuzzer_data.statistics.exit_farm_misses.get() + 1;
            fuzzer_data.statistics.exit_farm_misses.set(statistic_value);

            return;
        }

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_transfer(
            &caller,
            &farm_setup.farm_wrapper,
            farm_token_id,
            farm_token_nonce,
            &farm_out_amount.clone(),
            |sc| {
                sc.exit_farm(OptionalArg::None);

                StateChange::Commit
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            let statistic_value = fuzzer_data.statistics.exit_farm_hits.get() + 1;
            fuzzer_data.statistics.exit_farm_hits.set(statistic_value);
        } else {
            println!("Exit farm error: {}", tx_result_string);
            let statistic_value = fuzzer_data.statistics.exit_farm_misses.get() + 1;
            fuzzer_data.statistics.exit_farm_misses.set(statistic_value);
        }
    }

    pub fn claim_rewards<PairObjBuilder, FarmObjBuilder>(
        fuzzer_data: &mut FuzzerData<PairObjBuilder, FarmObjBuilder>,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let farm_token_nonce = 1u64;

        let farm_setup = fuzzer_data.farms.choose(&mut rand::thread_rng()).unwrap();

        let farm_token_id = farm_setup.farm_token.as_bytes();

        let mut rng = rand::thread_rng();
        let seed = rng.gen_range(0..fuzzer_data.fuzz_args.claim_rewards_max_value) + 1;

        let farm_token_amount = rust_biguint!(seed);

        let caller = fuzzer_data.users.choose(&mut rand::thread_rng()).unwrap();

        // fuzzer_data.blockchain_wrapper.set_block_epoch(5);
        // fuzzer_data.blockchain_wrapper.set_block_nonce(10);

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
            let statistic_value = fuzzer_data.statistics.claim_rewards_hits.get() + 1;
            fuzzer_data
                .statistics
                .claim_rewards_hits
                .set(statistic_value);
        } else {
            println!("Claim rewards error: {}", tx_result_string);
            let statistic_value = fuzzer_data.statistics.claim_rewards_misses.get() + 1;
            fuzzer_data
                .statistics
                .claim_rewards_misses
                .set(statistic_value);
        }
    }

    pub fn compound_rewards<PairObjBuilder, FarmObjBuilder>(
        fuzzer_data: &mut FuzzerData<PairObjBuilder, FarmObjBuilder>,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let farm_token_nonce = 1u64;

        let farm_setup = fuzzer_data.farms.choose(&mut rand::thread_rng()).unwrap();

        let farm_token_id = farm_setup.farm_token.as_bytes();

        let mut rng = rand::thread_rng();
        let seed = rng.gen_range(0..fuzzer_data.fuzz_args.compound_rewards_max_value) + 1;

        let farm_token_amount = rust_biguint!(seed);

        let caller = fuzzer_data.users.choose(&mut rand::thread_rng()).unwrap();

        // fuzzer_data.blockchain_wrapper.set_block_epoch(5);
        // fuzzer_data.blockchain_wrapper.set_block_nonce(10);

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
            let statistic_value = fuzzer_data.statistics.compound_rewards_hits.get() + 1;
            fuzzer_data
                .statistics
                .compound_rewards_hits
                .set(statistic_value);
        } else {
            println!("Compound rewards error: {}", tx_result_string);
            let statistic_value = fuzzer_data.statistics.compound_rewards_misses.get() + 1;
            fuzzer_data
                .statistics
                .compound_rewards_misses
                .set(statistic_value);
        }
    }
}
