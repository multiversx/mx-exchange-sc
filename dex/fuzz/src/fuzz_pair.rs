#[cfg(test)]
pub mod fuzz_pair_test {
    #![allow(deprecated)]

    multiversx_sc::imports!();
    multiversx_sc::derive_imports!();

    use multiversx_sc_scenario::{
        managed_biguint, managed_token_id, rust_biguint, whitebox_legacy::TxTokenTransfer, DebugApi,
    };

    use rand::prelude::*;

    use crate::fuzz_data::fuzz_data_tests::*;
    use pair::*;

    pub fn add_liquidity<PairObjBuilder, FarmObjBuilder, FactoryObjBuilder, PriceDiscObjBuilder>(
        fuzzer_data: &mut FuzzerData<
            PairObjBuilder,
            FarmObjBuilder,
            FactoryObjBuilder,
            PriceDiscObjBuilder,
        >,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
        FactoryObjBuilder: 'static + Copy + Fn() -> factory::ContractObj<DebugApi>,
        PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    {
        let pair_index = fuzzer_data.rng.gen_range(0..fuzzer_data.swap_pairs.len());
        let caller_index = fuzzer_data.rng.gen_range(0..fuzzer_data.users.len());

        let caller = &fuzzer_data.users[caller_index];
        let swap_pair = &mut fuzzer_data.swap_pairs[pair_index];

        let first_token = swap_pair.first_token.as_bytes();
        let second_token = swap_pair.second_token.as_bytes();
        let lp_token = swap_pair.lp_token.as_bytes();

        let seed = fuzzer_data
            .rng
            .gen_range(0..fuzzer_data.fuzz_args.add_liquidity_max_value)
            + 1;

        let first_token_amount = seed;
        let second_token_amount = seed;
        let first_token_min = seed / 100;
        let second_token_min = seed / 100;

        let first_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, first_token, 0);
        let second_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, second_token, 0);
        let lp_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, lp_token, 0);

        if first_token_before < rust_biguint!(first_token_amount)
            || second_token_before < rust_biguint!(second_token_amount)
        {
            println!("Add liquidity error: Not enough token user balance");
            fuzzer_data.statistics.add_liquidity_misses += 1;

            return;
        }

        let payments = vec![
            TxTokenTransfer {
                token_identifier: first_token.to_vec(),
                nonce: 0,
                value: rust_biguint!(first_token_amount),
            },
            TxTokenTransfer {
                token_identifier: second_token.to_vec(),
                nonce: 0,
                value: rust_biguint!(second_token_amount),
            },
        ];

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_multi_transfer(
            &caller.address,
            &swap_pair.pair_wrapper,
            &payments,
            |sc| {
                sc.add_liquidity(
                    managed_biguint!(first_token_min),
                    managed_biguint!(second_token_min),
                );
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            fuzzer_data.statistics.add_liquidity_hits += 1;

            let first_token_after =
                fuzzer_data
                    .blockchain_wrapper
                    .get_esdt_balance(&caller.address, first_token, 0);
            let second_token_after =
                fuzzer_data
                    .blockchain_wrapper
                    .get_esdt_balance(&caller.address, second_token, 0);
            let lp_token_after =
                fuzzer_data
                    .blockchain_wrapper
                    .get_esdt_balance(&caller.address, lp_token, 0);

            if first_token_after > first_token_before || second_token_after > second_token_before {
                println!("Add liquidity warning: Wrong final tokens balances");
            } else if lp_token_after < lp_token_before {
                println!("Add liquidity warning: Wrong lp token balance");
            }
        } else {
            println!("Add liquidity error: {}", tx_result_string);
            fuzzer_data.statistics.add_liquidity_misses += 1;
        }
    }

    pub fn remove_liquidity<
        PairObjBuilder,
        FarmObjBuilder,
        FactoryObjBuilder,
        PriceDiscObjBuilder,
    >(
        fuzzer_data: &mut FuzzerData<
            PairObjBuilder,
            FarmObjBuilder,
            FactoryObjBuilder,
            PriceDiscObjBuilder,
        >,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
        FactoryObjBuilder: 'static + Copy + Fn() -> factory::ContractObj<DebugApi>,
        PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    {
        let pair_index = fuzzer_data.rng.gen_range(0..fuzzer_data.swap_pairs.len());
        let caller_index = fuzzer_data.rng.gen_range(0..fuzzer_data.users.len());

        let caller = &fuzzer_data.users[caller_index];
        let swap_pair = &mut fuzzer_data.swap_pairs[pair_index];

        let first_token = swap_pair.first_token.as_bytes();
        let second_token = swap_pair.second_token.as_bytes();
        let lp_token = swap_pair.lp_token.as_bytes();

        let seed = fuzzer_data
            .rng
            .gen_range(0..fuzzer_data.fuzz_args.remove_liquidity_max_value)
            + 1;

        let lp_token_amount = seed;
        let first_token_min = seed / 100;
        let second_token_min = seed / 100;

        let first_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, first_token, 0);
        let second_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, second_token, 0);
        let lp_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, lp_token, 0);

        if lp_token_before < rust_biguint!(lp_token_amount) {
            println!("Remove liquidity error: Not enough LP token user balance");
            fuzzer_data.statistics.remove_liquidity_misses += 1;

            return;
        }

        let payments = vec![TxTokenTransfer {
            token_identifier: lp_token.to_vec(),
            nonce: 0,
            value: rust_biguint!(lp_token_amount),
        }];

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_multi_transfer(
            &caller.address,
            &swap_pair.pair_wrapper,
            &payments,
            |sc| {
                sc.remove_liquidity(
                    managed_biguint!(first_token_min),
                    managed_biguint!(second_token_min),
                );
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            fuzzer_data.statistics.remove_liquidity_hits += 1;

            let first_token_after =
                fuzzer_data
                    .blockchain_wrapper
                    .get_esdt_balance(&caller.address, first_token, 0);
            let second_token_after =
                fuzzer_data
                    .blockchain_wrapper
                    .get_esdt_balance(&caller.address, second_token, 0);
            let lp_token_after =
                fuzzer_data
                    .blockchain_wrapper
                    .get_esdt_balance(&caller.address, lp_token, 0);

            if first_token_after < first_token_before || second_token_after < second_token_before {
                println!("Remove liquidity warning: Wrong final tokens balances");
            } else if lp_token_after > lp_token_before {
                println!("Remove liquidity warning: Wrong lp token balance");
            }
        } else {
            println!("Remove liquidity error: {}", tx_result_string);
            fuzzer_data.statistics.remove_liquidity_misses += 1;
        }
    }

    pub fn swap_pair<PairObjBuilder, FarmObjBuilder, FactoryObjBuilder, PriceDiscObjBuilder>(
        fuzzer_data: &mut FuzzerData<
            PairObjBuilder,
            FarmObjBuilder,
            FactoryObjBuilder,
            PriceDiscObjBuilder,
        >,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
        FactoryObjBuilder: 'static + Copy + Fn() -> factory::ContractObj<DebugApi>,
        PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    {
        let pair_index = fuzzer_data.rng.gen_range(0..fuzzer_data.swap_pairs.len());
        let caller_index = fuzzer_data.rng.gen_range(0..fuzzer_data.users.len());

        let caller = &fuzzer_data.users[caller_index];
        let swap_pair = &mut fuzzer_data.swap_pairs[pair_index];

        let payment_token_id = swap_pair.first_token.as_bytes();
        let desired_token_id = swap_pair.second_token.as_bytes();

        let payment_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, payment_token_id, 0);

        let desired_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, desired_token_id, 0);

        let seed = fuzzer_data
            .rng
            .gen_range(0..fuzzer_data.fuzz_args.swap_max_value)
            + 1;

        let payment_amount = seed;
        let payment_amount_max = seed * 10;
        let desired_amount_min = seed / 100;

        let swap_fixed_input: bool = fuzzer_data.rng.gen();

        if swap_fixed_input {
            if payment_token_before < rust_biguint!(payment_amount) {
                println!("Swap fixed input error: Not enough payment token user balance");
                fuzzer_data.statistics.swap_fixed_input_misses += 1;

                return;
            }

            let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_transfer(
                &caller.address,
                &swap_pair.pair_wrapper,
                payment_token_id,
                0,
                &rust_biguint!(payment_amount),
                |sc| {
                    sc.swap_tokens_fixed_input(
                        managed_token_id!(desired_token_id),
                        managed_biguint!(desired_amount_min),
                    );
                },
            );

            let tx_result_string = tx_result.result_message;

            let payment_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
                &caller.address,
                payment_token_id,
                0,
            );

            let desired_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
                &caller.address,
                desired_token_id,
                0,
            );

            if payment_token_after > payment_token_before {
                println!("Swap fixed input error: final payment token balance is higher than the initial balance");
                fuzzer_data.statistics.swap_fixed_input_misses += 1;
            } else if desired_token_after < desired_token_before {
                println!("Swap fixed input error: wrong desired token amount");
                fuzzer_data.statistics.swap_fixed_input_misses += 1;
            } else if tx_result_string.trim().is_empty() {
                fuzzer_data.statistics.swap_fixed_input_hits += 1;
            } else {
                println!("Swap fixed input error: {}", tx_result_string);
                fuzzer_data.statistics.swap_fixed_input_misses += 1;
            }
        } else {
            //swap fixed output
            if payment_token_before < rust_biguint!(payment_amount_max) {
                println!("Swap fixed output error: Not enough token user balance");
                fuzzer_data.statistics.swap_fixed_output_misses += 1;
                return;
            }

            let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_transfer(
                &caller.address,
                &swap_pair.pair_wrapper,
                payment_token_id,
                0,
                &rust_biguint!(payment_amount),
                |sc| {
                    sc.swap_tokens_fixed_output(
                        managed_token_id!(desired_token_id),
                        managed_biguint!(desired_amount_min),
                    );
                },
            );

            let tx_result_string = tx_result.result_message;

            let payment_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
                &caller.address,
                payment_token_id,
                0,
            );

            let desired_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
                &caller.address,
                desired_token_id,
                0,
            );

            if payment_token_after > payment_token_before {
                println!("Swap fixed output error: final payment token balance is higher than the initial balance");
                fuzzer_data.statistics.swap_fixed_output_misses += 1;
            } else if desired_token_after < desired_token_before {
                println!("Swap fixed output error: wrong desired token amount");
                fuzzer_data.statistics.swap_fixed_output_misses += 1;
            } else if tx_result_string.trim().is_empty() {
                fuzzer_data.statistics.swap_fixed_output_hits += 1;
            } else {
                println!("Swap fixed output error: {}", tx_result_string);
                fuzzer_data.statistics.swap_fixed_output_misses += 1;
            }
        }
    }
}
