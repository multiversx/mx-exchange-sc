#[cfg(test)]
mod test {
    #![allow(deprecated)]

    multiversx_sc::imports!();
    multiversx_sc::derive_imports!();

    use std::time::SystemTime;

    use crate::fuzz_data::fuzz_data_tests::*;
    use crate::fuzz_factory::fuzz_factory_test::*;
    use crate::fuzz_farm::fuzz_farm_test::*;
    use crate::fuzz_pair::fuzz_pair_test::*;
    use crate::fuzz_price_discovery::fuzz_price_discovery_test::*;

    use multiversx_sc_scenario::DebugApi;

    use rand::distributions::weighted::WeightedIndex;
    use rand::prelude::*;
    use std::time::UNIX_EPOCH;

    #[test]
    fn start_fuzzer() {
        // Random seed based on current time - can be given a specific value for a predetermined fuzz scenario
        let seed_base = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Incorrect output");
        let seed = seed_base.as_secs() * 1000 + seed_base.subsec_nanos() as u64 / 1_000_000; //in ms

        let mut fuzzer_data = FuzzerData::new(
            seed,
            pair::contract_obj,
            farm::contract_obj,
            factory::contract_obj,
            price_discovery::contract_obj,
        );

        println!("Started fuzz testing with seed: {}", (seed));

        let choices = [
            (1, fuzzer_data.fuzz_args.add_liquidity_prob),
            (2, fuzzer_data.fuzz_args.remove_liquidity_prob),
            (3, fuzzer_data.fuzz_args.swap_prob),
            (4, fuzzer_data.fuzz_args.enter_farm_prob),
            (5, fuzzer_data.fuzz_args.exit_farm_prob),
            (6, fuzzer_data.fuzz_args.claim_rewards_prob),
            (7, fuzzer_data.fuzz_args.compound_rewards_prob),
            (8, fuzzer_data.fuzz_args.factory_lock_asset_prob),
            (9, fuzzer_data.fuzz_args.factory_unlock_asset_prob),
            (10, fuzzer_data.fuzz_args.price_discovery_deposit_prob),
            (11, fuzzer_data.fuzz_args.price_discovery_withdraw_prob),
            (12, fuzzer_data.fuzz_args.price_discovery_redeem_prob),
        ];

        let mut block_epoch = 1;
        fuzzer_data.blockchain_wrapper.set_block_nonce(block_epoch);

        for block_nonce in 1..=fuzzer_data.fuzz_args.num_events {
            let choice_index = WeightedIndex::new(choices.iter().map(|choice| choice.1)).unwrap();
            let random_choice = choices[choice_index.sample(&mut fuzzer_data.rng)].0;

            // custom logic to enforce blockchain time passing (+1 epoch for each 2 blocks)
            // does not reflect an accurate time passing in a real blockchain
            fuzzer_data.blockchain_wrapper.set_block_nonce(block_nonce);
            if block_nonce % 2 == 0 {
                block_epoch += fuzzer_data.fuzz_args.block_nonce_increase;
                fuzzer_data.blockchain_wrapper.set_block_epoch(block_epoch);
            }

            match random_choice {
                1 => {
                    println!("Event no. {}: Add liquidity", (block_nonce));
                    add_liquidity(&mut fuzzer_data);
                }
                2 => {
                    println!("Event no. {}: Remove liquidity", (block_nonce));
                    remove_liquidity(&mut fuzzer_data);
                }
                3 => {
                    println!("Event no. {}: Swap pair", (block_nonce));
                    swap_pair(&mut fuzzer_data);
                }
                4 => {
                    println!("Event no. {}: Enter farm", (block_nonce));
                    enter_farm(&mut fuzzer_data);
                }
                5 => {
                    println!("Event no. {}: Exit farm", (block_nonce));
                    exit_farm(&mut fuzzer_data);
                }
                6 => {
                    println!("Event no. {}: Claim reward", (block_nonce));
                    claim_rewards(&mut fuzzer_data);
                }
                7 => {
                    println!("Event no. {}: Compound reward", (block_nonce));
                    compound_rewards(&mut fuzzer_data);
                }
                8 => {
                    println!("Event no. {}: Factory lock tokens", (block_nonce));
                    lock_assets(&mut fuzzer_data);
                }
                9 => {
                    println!("Event no. {}: Factory unlock tokens", (block_nonce));
                    unlock_assets(&mut fuzzer_data);
                }
                10 => {
                    println!("Event no. {}: Price discovery deposit", (block_nonce));
                    price_discovery_deposit(&mut fuzzer_data);
                }
                11 => {
                    println!("Event no. {}: Price discovery withdraw", (block_nonce));
                    price_discovery_withdraw(&mut fuzzer_data);
                }
                12 => {
                    println!("Event no. {}: Price discovery redeem", (block_nonce));
                    price_discovery_redeem(&mut fuzzer_data);
                }
                _ => println!("No event triggered"),
            }
        }

        print_statistics(&mut fuzzer_data, seed);
    }

    fn print_statistics<PairObjBuilder, FarmObjBuilder, FactoryObjBuilder, PriceDiscObjBuilder>(
        fuzzer_data: &mut FuzzerData<
            PairObjBuilder,
            FarmObjBuilder,
            FactoryObjBuilder,
            PriceDiscObjBuilder,
        >,
        seed: u64,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
        FactoryObjBuilder: 'static + Copy + Fn() -> factory::ContractObj<DebugApi>,
        PriceDiscObjBuilder: 'static + Copy + Fn() -> price_discovery::ContractObj<DebugApi>,
    {
        println!();
        println!("Statistics:");
        println!(
            "Total number of events: {}",
            fuzzer_data.fuzz_args.num_events
        );
        println!("Random seed used: {}", seed);
        println!();
        println!(
            "swapFixedInputHits: {}",
            fuzzer_data.statistics.swap_fixed_input_hits
        );
        println!(
            "swapFixedInputMisses: {}",
            fuzzer_data.statistics.swap_fixed_input_misses
        );
        println!();
        println!(
            "swapFixedOutputHits: {}",
            fuzzer_data.statistics.swap_fixed_output_hits
        );
        println!(
            "swapFixedOutputMissed: {}",
            fuzzer_data.statistics.swap_fixed_output_misses
        );
        println!();
        println!(
            "addLiquidityHits: {}",
            fuzzer_data.statistics.add_liquidity_hits
        );
        println!(
            "addLiquidityMisses: {}",
            fuzzer_data.statistics.add_liquidity_misses
        );
        println!();
        println!(
            "removeLiquidityHits: {}",
            fuzzer_data.statistics.remove_liquidity_hits
        );
        println!(
            "removeLiquidityMisses: {}",
            fuzzer_data.statistics.remove_liquidity_misses
        );
        println!();
        println!("enterFarmHits: {}", fuzzer_data.statistics.enter_farm_hits);
        println!(
            "enterFarmMisses: {}",
            fuzzer_data.statistics.enter_farm_misses
        );
        println!();
        println!("exitFarmHits: {}", fuzzer_data.statistics.exit_farm_hits);
        println!(
            "exitFarmMisses: {}",
            fuzzer_data.statistics.exit_farm_misses
        );
        println!(
            "exitFarmWithRewards: {}",
            fuzzer_data.statistics.exit_farm_with_rewards
        );
        println!();
        println!(
            "claimRewardsHits: {}",
            fuzzer_data.statistics.claim_rewards_hits
        );
        println!(
            "claimRewardsMisses: {}",
            fuzzer_data.statistics.claim_rewards_misses
        );
        println!(
            "claimRewardsWithRewards: {}",
            fuzzer_data.statistics.claim_rewards_with_rewards
        );
        println!();
        println!(
            "compoundRewardsHits: {}",
            fuzzer_data.statistics.compound_rewards_hits
        );
        println!(
            "compoundRewardsMisses: {}",
            fuzzer_data.statistics.compound_rewards_misses
        );
        println!();
        println!(
            "factoryLockHits: {}",
            fuzzer_data.statistics.factory_lock_hits
        );
        println!(
            "factoryLockMisses: {}",
            fuzzer_data.statistics.factory_lock_misses
        );
        println!();
        println!(
            "factoryUnlockHits: {}",
            fuzzer_data.statistics.factory_unlock_hits
        );
        println!(
            "factoryUnlockMisses: {}",
            fuzzer_data.statistics.factory_unlock_misses
        );
        println!();
        println!(
            "priceDiscoveryDepositHits: {}",
            fuzzer_data.statistics.price_discovery_deposit_hits
        );
        println!(
            "priceDiscoveryDepositMisses: {}",
            fuzzer_data.statistics.price_discovery_deposit_misses
        );
        println!();
        println!(
            "priceDiscoveryWithdrawHits: {}",
            fuzzer_data.statistics.price_discovery_withdraw_hits
        );
        println!(
            "priceDiscoveryWithdrawMisses: {}",
            fuzzer_data.statistics.price_discovery_withdraw_misses
        );
        println!();
        println!(
            "priceDiscoveryRedeemHits: {}",
            fuzzer_data.statistics.price_discovery_redeem_hits
        );
        println!(
            "priceDiscoveryRedeemMisses: {}",
            fuzzer_data.statistics.price_discovery_redeem_misses
        );
        println!();
    }
}
