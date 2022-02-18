#[cfg(test)]
pub mod fuzz_pair_test {

    elrond_wasm::imports!();
    elrond_wasm::derive_imports!();

    use elrond_wasm::types::{BigUint, OptionalArg, TokenIdentifier};
    use elrond_wasm_debug::{
        managed_biguint, managed_token_id, rust_biguint, testing_framework::*,
        tx_mock::TxInputESDT, DebugApi,
    };

    use rand::prelude::SliceRandom;
    use rand::Rng;

    use crate::fuzz_data::fuzz_data_tests::*;
    use pair::*;

    pub fn add_liquidity<PairObjBuilder, FarmObjBuilder>(
        fuzzer_data: &mut FuzzerData<PairObjBuilder, FarmObjBuilder>,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let swap_pair = fuzzer_data
            .swap_pairs
            .choose(&mut rand::thread_rng())
            .unwrap();
        let caller = fuzzer_data.users.choose(&mut rand::thread_rng()).unwrap();

        let first_token = swap_pair.first_token.as_bytes();
        let second_token = swap_pair.second_token.as_bytes();

        let mut rng = rand::thread_rng();

        let seed = rng.gen_range(0..fuzzer_data.fuzz_args.add_liquidity_max_value) + 1;

        let first_token_amount = seed;
        let second_token_amount = seed;
        let first_token_min = seed / 100;
        let second_token_min = seed / 100;

        let first_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller, first_token, 0);
        let second_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller, second_token, 0);

        if first_token_before < rust_biguint!(first_token_amount)
            || second_token_before < rust_biguint!(second_token_amount)
        {
            println!("Not enough token user balance");
            let statistic_value = fuzzer_data.statistics.add_liquidity_misses.get() + 1;
            fuzzer_data
                .statistics
                .add_liquidity_misses
                .set(statistic_value);

            return;
        }

        let payments = vec![
            TxInputESDT {
                token_identifier: first_token.to_vec(),
                nonce: 0,
                value: rust_biguint!(first_token_amount),
            },
            TxInputESDT {
                token_identifier: second_token.to_vec(),
                nonce: 0,
                value: rust_biguint!(second_token_amount),
            },
        ];

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_multi_transfer(
            &caller,
            &swap_pair.pair_wrapper,
            &payments,
            |sc| {
                sc.add_liquidity(
                    managed_biguint!(first_token_min),
                    managed_biguint!(second_token_min),
                    OptionalArg::None,
                );

                StateChange::Commit
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            let statistic_value = fuzzer_data.statistics.add_liquidity_hits.get() + 1;
            fuzzer_data
                .statistics
                .add_liquidity_hits
                .set(statistic_value);
        } else {
            println!("Add liquidity error: {}", tx_result_string);
            let statistic_value = fuzzer_data.statistics.add_liquidity_misses.get() + 1;
            fuzzer_data
                .statistics
                .add_liquidity_misses
                .set(statistic_value);
        }
    }

    pub fn remove_liquidity<PairObjBuilder, FarmObjBuilder>(
        fuzzer_data: &mut FuzzerData<PairObjBuilder, FarmObjBuilder>,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let swap_pair = fuzzer_data
            .swap_pairs
            .choose(&mut rand::thread_rng())
            .unwrap();
        let caller = fuzzer_data.users.choose(&mut rand::thread_rng()).unwrap();

        let lp_token = swap_pair.lp_token.as_bytes();

        let mut rng = rand::thread_rng();

        let seed = rng.gen_range(0..fuzzer_data.fuzz_args.remove_liquidity_max_value) + 1;

        let lp_token_amount = seed;
        let first_token_min = seed / 100;
        let second_token_min = seed / 100;

        let lp_token_before = fuzzer_data
            .blockchain_wrapper
            .get_esdt_balance(&caller, lp_token, 0);

        if lp_token_before < rust_biguint!(lp_token_amount) {
            println!("Not enough LP token user balance");
            let statistic_value = fuzzer_data.statistics.remove_liquidity_misses.get() + 1;
            fuzzer_data
                .statistics
                .remove_liquidity_misses
                .set(statistic_value);

            return;
        }

        let payments = vec![TxInputESDT {
            token_identifier: lp_token.to_vec(),
            nonce: 0,
            value: rust_biguint!(lp_token_amount),
        }];

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_multi_transfer(
            &caller,
            &swap_pair.pair_wrapper,
            &payments,
            |sc| {
                sc.remove_liquidity(
                    managed_token_id!(lp_token),
                    0,
                    managed_biguint!(lp_token_amount),
                    managed_biguint!(first_token_min),
                    managed_biguint!(second_token_min),
                    OptionalArg::None,
                );

                StateChange::Commit
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            let statistic_value = fuzzer_data.statistics.remove_liquidity_hits.get() + 1;
            fuzzer_data
                .statistics
                .remove_liquidity_hits
                .set(statistic_value);
        } else {
            println!("Remove liquidity error: {}", tx_result_string);
            let statistic_value = fuzzer_data.statistics.remove_liquidity_misses.get() + 1;
            fuzzer_data
                .statistics
                .remove_liquidity_misses
                .set(statistic_value);
        }
    }

    pub fn swap_pair<PairObjBuilder, FarmObjBuilder>(
        fuzzer_data: &mut FuzzerData<PairObjBuilder, FarmObjBuilder>,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let swap_pair = fuzzer_data
            .swap_pairs
            .choose(&mut rand::thread_rng())
            .unwrap();

        let caller = fuzzer_data.users.choose(&mut rand::thread_rng()).unwrap();

        let payment_token_id = swap_pair.first_token.as_bytes();
        let desired_token_id = swap_pair.second_token.as_bytes();

        let first_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller, payment_token_id, 0);

        let mut rng = rand::thread_rng();

        let seed = rng.gen_range(0..fuzzer_data.fuzz_args.swap_max_value) + 1;

        let payment_amount = seed;
        let desired_amount = seed;
        let payment_amount_max = seed * 10;
        let desired_amount_min = seed / 100;

        let swap_input: bool = rng.gen();

        if swap_input {
            if first_token_before < rust_biguint!(payment_amount) {
                println!("Not enough payment token user balance");
                let statistic_value = fuzzer_data.statistics.swap_fixed_input_misses.get() + 1;
                fuzzer_data
                    .statistics
                    .swap_fixed_input_misses
                    .set(statistic_value);

                return;
            }

            let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_multi_transfer(
                &caller,
                &swap_pair.pair_wrapper,
                &vec![],
                |sc| {
                    sc.swap_tokens_fixed_input(
                        managed_token_id!(payment_token_id),
                        0,
                        managed_biguint!(payment_amount),
                        managed_token_id!(desired_token_id),
                        managed_biguint!(desired_amount_min),
                        OptionalArg::None,
                    );

                    StateChange::Commit
                },
            );

            let tx_result_string = tx_result.result_message;

            if tx_result_string.trim().is_empty() {
                let statistic_value = fuzzer_data.statistics.swap_fixed_input_hits.get() + 1;
                fuzzer_data
                    .statistics
                    .swap_fixed_input_hits
                    .set(statistic_value);
            } else {
                println!("Swap fixed input error: {}", tx_result_string);
                let statistic_value = fuzzer_data.statistics.swap_fixed_input_misses.get() + 1;
                fuzzer_data
                    .statistics
                    .swap_fixed_input_misses
                    .set(statistic_value);
            }
        } else {
            //swap output
            if first_token_before < rust_biguint!(payment_amount_max) {
                println!("Not enough token user balance");
                let statistic_value = fuzzer_data.statistics.add_liquidity_misses.get() + 1;
                fuzzer_data
                    .statistics
                    .add_liquidity_misses
                    .set(statistic_value);
                return;
            }

            let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_multi_transfer(
                &caller,
                &swap_pair.pair_wrapper,
                &vec![],
                |sc| {
                    sc.swap_tokens_fixed_output(
                        managed_token_id!(payment_token_id),
                        0,
                        managed_biguint!(payment_amount_max),
                        managed_token_id!(desired_token_id),
                        managed_biguint!(desired_amount),
                        OptionalArg::None,
                    );

                    StateChange::Commit
                },
            );

            let tx_result_string = tx_result.result_message;

            if tx_result_string.trim().is_empty() {
                let statistic_value = fuzzer_data.statistics.swap_fixed_output_hits.get() + 1;
                fuzzer_data
                    .statistics
                    .swap_fixed_output_hits
                    .set(statistic_value);
            } else {
                println!("Swap fixed output error: {}", tx_result_string);
                let statistic_value = fuzzer_data.statistics.swap_fixed_output_misses.get() + 1;
                fuzzer_data
                    .statistics
                    .swap_fixed_output_misses
                    .set(statistic_value);
            }
        }
    }
}
