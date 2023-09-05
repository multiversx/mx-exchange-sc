#[cfg(test)]
pub mod fuzz_farm_test {
    #![allow(deprecated)]

    multiversx_sc::imports!();
    multiversx_sc::derive_imports!();

    use std::cmp::Ordering;

    use multiversx_sc_scenario::whitebox_legacy::TxTokenTransfer;
    use multiversx_sc_scenario::{managed_biguint, rust_biguint, DebugApi};

    use crate::fuzz_data::fuzz_data_tests::*;
    use farm::*;

    use rand::prelude::*;

    pub fn enter_farm<PairObjBuilder, FarmObjBuilder, FactoryObjBuilder, PriceDiscObjBuilder>(
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
        let rust_zero = rust_biguint!(0u64);

        let farm_index = fuzzer_data.rng.gen_range(0..fuzzer_data.farms.len());
        let caller_index = fuzzer_data.rng.gen_range(0..fuzzer_data.users.len());

        let caller = &fuzzer_data.users[caller_index];
        let farm_setup = &mut fuzzer_data.farms[farm_index];
        let farm_nonce = farm_setup.farm_nonce.get();

        let lp_token_id = farm_setup.farming_token.as_bytes();
        let farm_token_id = farm_setup.farm_token.as_bytes();

        let seed = fuzzer_data
            .rng
            .gen_range(0..fuzzer_data.fuzz_args.enter_farm_max_value)
            + 1;

        let farm_in_amount = rust_biguint!(seed);

        let lp_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, lp_token_id, 0);

        if lp_token_before < farm_in_amount {
            println!("Enter farm error: Not enough LP token user balance");
            fuzzer_data.statistics.enter_farm_misses += 1;

            return;
        }

        let mut payments = Vec::new();
        payments.push(TxTokenTransfer {
            token_identifier: lp_token_id.to_vec(),
            nonce: 0,
            value: farm_in_amount,
        });

        //randomly add all existing farm positions for merge
        let merge_farm_positions: bool = fuzzer_data.rng.gen();
        if merge_farm_positions && farm_setup.farmer_info.get(&caller.address).is_some() {
            for farm_token_nonce in farm_setup.farmer_info.get(&caller.address).unwrap().iter() {
                let farm_token_amount = fuzzer_data.blockchain_wrapper.get_esdt_balance(
                    &caller.address,
                    farm_token_id,
                    *farm_token_nonce,
                );

                if farm_token_amount > rust_zero {
                    payments.push(TxTokenTransfer {
                        token_identifier: farm_token_id.to_vec(),
                        nonce: *farm_token_nonce,
                        value: farm_token_amount.clone(),
                    });
                }
            }
        }

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_multi_transfer(
            &caller.address,
            &farm_setup.farm_wrapper,
            &payments,
            |sc| {
                sc.enter_farm_endpoint(OptionalValue::None);
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            fuzzer_data.statistics.enter_farm_hits += 1;

            // Clear previous farm positions
            if merge_farm_positions {
                farm_setup.farmer_info.remove(&caller.address);
            }

            // Update farm nonce
            farm_setup
                .farmer_info
                .entry(caller.address.clone())
                .or_default()
                .push(farm_nonce);
            farm_setup.farm_nonce.set(farm_nonce + 1);
        } else {
            println!("Enter farm errors: {}", tx_result_string);
            fuzzer_data.statistics.enter_farm_misses += 1;
        }
    }

    pub fn exit_farm<PairObjBuilder, FarmObjBuilder, FactoryObjBuilder, PriceDiscObjBuilder>(
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
        let rust_zero = rust_biguint!(0u64);

        let farm_index = fuzzer_data.rng.gen_range(0..fuzzer_data.farms.len());
        let caller_index = fuzzer_data.rng.gen_range(0..fuzzer_data.users.len());

        let caller = &fuzzer_data.users[caller_index];
        let farm_setup = &mut fuzzer_data.farms[farm_index];

        let farm_token_id = farm_setup.farm_token.as_bytes();
        let reward_token_id = farm_setup.reward_token.as_bytes();

        let seed = fuzzer_data
            .rng
            .gen_range(0..fuzzer_data.fuzz_args.exit_farm_max_value)
            + 1;
        let mut farm_out_amount = rust_biguint!(seed);

        let farm_token_nonce = match farm_setup.farmer_info.get(&caller.address) {
            Some(s) => *s.choose(&mut fuzzer_data.rng).unwrap(),
            None => 0,
        };

        let farm_token_before = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            &caller.address,
            farm_token_id,
            farm_token_nonce,
        );

        let reward_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, reward_token_id, 0);

        if farm_token_before == rust_zero {
            println!("Exit farm error: Not enough farm token user balance");
            fuzzer_data.statistics.exit_farm_misses += 1;
            return;
        } else if farm_token_before < farm_out_amount {
            farm_out_amount = farm_token_before;
        }

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_transfer(
            &caller.address,
            &farm_setup.farm_wrapper,
            farm_token_id,
            farm_token_nonce,
            &farm_out_amount,
            |sc| {
                sc.exit_farm_endpoint(managed_biguint!(seed), OptionalValue::None);
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            fuzzer_data.statistics.exit_farm_hits += 1;

            // Check rewards
            let reward_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
                &caller.address,
                reward_token_id,
                0,
            );
            match reward_token_after.cmp(&reward_token_before) {
                Ordering::Greater => fuzzer_data.statistics.exit_farm_with_rewards += 1,
                Ordering::Less => {
                    println!("Exit farm warning: Lost reward tokens while exiting farm")
                }
                Ordering::Equal => {}
            }
        } else {
            println!("Exit farm error: {}", tx_result_string);
            fuzzer_data.statistics.exit_farm_misses += 1;
        }
    }

    pub fn claim_rewards<PairObjBuilder, FarmObjBuilder, FactoryObjBuilder, PriceDiscObjBuilder>(
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
        let rust_zero = rust_biguint!(0u64);

        let farm_index = fuzzer_data.rng.gen_range(0..fuzzer_data.farms.len());
        let caller_index = fuzzer_data.rng.gen_range(0..fuzzer_data.users.len());

        let caller = &fuzzer_data.users[caller_index];
        let farm_setup = &mut fuzzer_data.farms[farm_index];
        let farm_nonce = farm_setup.farm_nonce.get();

        let farm_token_id = farm_setup.farm_token.as_bytes();
        let reward_token_id = farm_setup.reward_token.as_bytes();

        let reward_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, reward_token_id, 0);

        // When claiming rewards, the caller uses all his farming positions
        let mut farm_token_amount_check = rust_biguint!(0u64);
        let mut payments = Vec::new();
        if farm_setup.farmer_info.get(&caller.address).is_some() {
            for farm_token_nonce in farm_setup.farmer_info.get(&caller.address).unwrap().iter() {
                let farm_token_amount = fuzzer_data.blockchain_wrapper.get_esdt_balance(
                    &caller.address,
                    farm_token_id,
                    *farm_token_nonce,
                );

                if farm_token_amount > rust_zero {
                    payments.push(TxTokenTransfer {
                        token_identifier: farm_token_id.to_vec(),
                        nonce: *farm_token_nonce,
                        value: farm_token_amount.clone(),
                    });

                    farm_token_amount_check += farm_token_amount;
                }
            }
        }

        if farm_token_amount_check == rust_zero {
            println!("Claim rewards error: Not enough farm token user balance");
            fuzzer_data.statistics.claim_rewards_misses += 1;
            return;
        }

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_multi_transfer(
            &caller.address,
            &farm_setup.farm_wrapper,
            &payments,
            |sc| {
                sc.claim_rewards_endpoint(OptionalValue::None);
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            fuzzer_data.statistics.claim_rewards_hits += 1;

            // Check rewards
            let reward_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
                &caller.address,
                reward_token_id,
                0,
            );
            match reward_token_after.cmp(&reward_token_before) {
                Ordering::Greater => fuzzer_data.statistics.claim_rewards_with_rewards += 1,
                Ordering::Less => {
                    println!("Claim rewards warning: Lost reward tokens while claiming rewards")
                }
                Ordering::Equal => {}
            }

            // Clear previous farm positions
            farm_setup.farmer_info.remove(&caller.address);

            // Update farm nonce
            farm_setup
                .farmer_info
                .entry(caller.address.clone())
                .or_default()
                .push(farm_nonce);
            farm_setup.farm_nonce.set(farm_nonce + 1);
        } else {
            println!("Claim rewards error: {}", tx_result_string);
            fuzzer_data.statistics.claim_rewards_misses += 1;
        }
    }

    pub fn compound_rewards<
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
        let rust_zero = rust_biguint!(0u64);

        let farm_index = fuzzer_data.rng.gen_range(0..fuzzer_data.farms.len());
        let caller_index = fuzzer_data.rng.gen_range(0..fuzzer_data.users.len());

        let caller = &fuzzer_data.users[caller_index];
        let farm_setup = &mut fuzzer_data.farms[farm_index];
        let farm_nonce = farm_setup.farm_nonce.get();

        let farm_token_id = farm_setup.farm_token.as_bytes();
        let farming_token_id = farm_setup.farming_token.as_bytes();
        let reward_token_id = farm_setup.reward_token.as_bytes();

        if farming_token_id != reward_token_id {
            println!("Compound rewards error: Farming token id is different from reward token id");
            fuzzer_data.statistics.compound_rewards_misses += 1;
            return;
        }

        // When compounding rewards, the caller uses all his farming positions
        let mut farm_token_amount_check = rust_biguint!(0u64);
        let mut payments = Vec::new();
        if farm_setup.farmer_info.get(&caller.address).is_some() {
            for farm_token_nonce in farm_setup.farmer_info.get(&caller.address).unwrap().iter() {
                let farm_token_amount = fuzzer_data.blockchain_wrapper.get_esdt_balance(
                    &caller.address,
                    farm_token_id,
                    *farm_token_nonce,
                );

                if farm_token_amount > rust_zero {
                    payments.push(TxTokenTransfer {
                        token_identifier: farm_token_id.to_vec(),
                        nonce: *farm_token_nonce,
                        value: farm_token_amount.clone(),
                    });

                    farm_token_amount_check += farm_token_amount;
                }
            }
        }

        if farm_token_amount_check == rust_zero {
            println!("Not enough farm token user balance");
            fuzzer_data.statistics.claim_rewards_misses += 1;

            return;
        }

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_multi_transfer(
            &caller.address,
            &farm_setup.farm_wrapper,
            &payments,
            |sc| {
                sc.compound_rewards_endpoint(OptionalValue::None);
            },
        );

        let tx_result_string = tx_result.result_message;

        if tx_result_string.trim().is_empty() {
            fuzzer_data.statistics.compound_rewards_hits += 1;

            // Clear previous farm positions
            farm_setup.farmer_info.remove(&caller.address);

            // Update farm nonce
            farm_setup
                .farmer_info
                .entry(caller.address.clone())
                .or_default()
                .push(farm_nonce);

            farm_setup.farm_nonce.set(farm_nonce + 1);
        } else {
            println!("Compound rewards error: {}", tx_result_string);
            fuzzer_data.statistics.compound_rewards_misses += 1;
        }
    }
}
