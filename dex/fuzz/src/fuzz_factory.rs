#[cfg(test)]
pub mod fuzz_factory_test {
    #![allow(deprecated)]

    multiversx_sc::imports!();
    multiversx_sc::derive_imports!();

    use multiversx_sc_scenario::whitebox_legacy::TxTokenTransfer;
    use multiversx_sc_scenario::{rust_biguint, DebugApi};

    use crate::fuzz_data::fuzz_data_tests::*;

    use factory::*;

    use rand::prelude::*;

    pub fn lock_assets<PairObjBuilder, FarmObjBuilder, FactoryObjBuilder, PriceDiscObjBuilder>(
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
        let caller_index = fuzzer_data.rng.gen_range(0..fuzzer_data.users.len());
        let caller = &mut fuzzer_data.users[caller_index];
        let factory_setup = &mut fuzzer_data.factory;

        let token_id = factory_setup.token.as_bytes();

        let seed = fuzzer_data
            .rng
            .gen_range(0..fuzzer_data.fuzz_args.factory_lock_asset_max_value)
            + 1;

        let amount_to_lock = rust_biguint!(seed);

        let token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, token_id, 0);

        if token_before < amount_to_lock {
            println!("Factory lock error: Not enough tokens");
            fuzzer_data.statistics.factory_lock_misses += 1;

            return;
        }

        let payments = vec![TxTokenTransfer {
            token_identifier: token_id.to_vec(),
            nonce: 0,
            value: amount_to_lock,
        }];

        let mut locked_asset_nonce = FACTORY_LOCK_NONCE;
        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_multi_transfer(
            &caller.address,
            &factory_setup.factory_wrapper,
            &payments,
            |sc| {
                let locked_assets = sc.lock_assets();

                locked_asset_nonce = locked_assets.token_nonce;
            },
        );

        if !caller.locked_asset_nonces.contains(&locked_asset_nonce) {
            caller.locked_asset_nonces.push(locked_asset_nonce);
        }

        let locked_amount = rust_biguint!(seed);

        let token_after =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, token_id, 0);

        let tx_result_string = tx_result.result_message;

        if !tx_result_string.trim().is_empty() {
            println!("Factory lock error: {}", tx_result_string);
            fuzzer_data.statistics.factory_lock_misses += 1;
        } else if token_after != token_before - &locked_amount {
            println!("Factory lock error: unlocked token final balance is incorrect");
            fuzzer_data.statistics.factory_lock_misses += 1;
        } else {
            fuzzer_data.statistics.factory_lock_hits += 1;
        }
    }

    pub fn unlock_assets<PairObjBuilder, FarmObjBuilder, FactoryObjBuilder, PriceDiscObjBuilder>(
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

        let caller_index = fuzzer_data.rng.gen_range(0..fuzzer_data.users.len());
        let caller = &mut fuzzer_data.users[caller_index];
        let factory_setup = &mut fuzzer_data.factory;

        let token_id = factory_setup.token.as_bytes();
        let locked_token_id = factory_setup.locked_token.as_bytes();

        // Choose a random locked token nonce to try to unlock
        let chosen_nonce = caller.locked_asset_nonces.choose(&mut fuzzer_data.rng);
        let locked_token_nonce = match chosen_nonce {
            Some(chosen_nonce) => *chosen_nonce,
            None => {
                println!("Factory unlock error: Caller does not have any locked tokens");
                fuzzer_data.statistics.factory_unlock_misses += 1;

                return;
            }
        };

        let seed = fuzzer_data
            .rng
            .gen_range(0..fuzzer_data.fuzz_args.factory_unlock_asset_max_value)
            + 1;

        let mut amount_to_unlock = rust_biguint!(seed);

        let token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, token_id, 0);

        let locked_token_before = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            &caller.address,
            locked_token_id,
            locked_token_nonce,
        );

        if locked_token_before < amount_to_unlock {
            if locked_token_before > rust_zero {
                amount_to_unlock = locked_token_before.clone();
            } else {
                println!("Factory unlock error: Not enough tokens");
                fuzzer_data.statistics.factory_unlock_misses += 1;

                return;
            }
        }

        let payments = vec![TxTokenTransfer {
            token_identifier: locked_token_id.to_vec(),
            nonce: locked_token_nonce,
            value: amount_to_unlock,
        }];

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_multi_transfer(
            &caller.address,
            &factory_setup.factory_wrapper,
            &payments,
            |sc| {
                sc.unlock_assets();
            },
        );

        let token_after =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, token_id, 0);

        let locked_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            &caller.address,
            locked_token_id,
            locked_token_nonce,
        );

        let unlocked_amount = rust_biguint!(seed);

        let tx_result_string = tx_result.result_message;

        if !tx_result_string.trim().is_empty() {
            println!("Factory unlock error: {}", tx_result_string);
            fuzzer_data.statistics.factory_unlock_misses += 1;
        } else if token_after != token_before + &unlocked_amount {
            println!("Factory unlock error: final balance is incorrect");
            fuzzer_data.statistics.factory_unlock_misses += 1;
        } else if locked_token_after != locked_token_before - &unlocked_amount {
            println!("Factory unlock error: locked token final balance is incorrect");
            fuzzer_data.statistics.factory_unlock_misses += 1;
        } else {
            fuzzer_data.statistics.factory_unlock_hits += 1;
        }
    }
}
