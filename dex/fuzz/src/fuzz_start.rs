
#[cfg(test)]
mod test {

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::fuzz_farm::fuzz_farm_test::*;
use crate::fuzz_pair::fuzz_pair_test::*;
use crate::fuzz_data::fuzz_data_tests::*;

use elrond_wasm_debug::{DebugApi, HashMap};

use rand::prelude::*;
use rand::distributions::weighted::WeightedIndex;

    #[test]
    fn start_fuzzer() {
        let mut fuzzer_data = FuzzerData::new(pair::contract_obj, farm::contract_obj);
        let mut farmer_info: HashMap<Address, Vec<u64>> = HashMap::new();

        let mut rng = thread_rng();
        let choices = [
            (1, fuzzer_data.fuzz_args.add_liquidity_prob),
            (2, fuzzer_data.fuzz_args.remove_liquidity_prob),
            (3, fuzzer_data.fuzz_args.swap_prob),
            (4, fuzzer_data.fuzz_args.enter_farm_prob),
            (5, fuzzer_data.fuzz_args.exit_farm_prob),
            (6, fuzzer_data.fuzz_args.claim_rewards_prob),
            (7, fuzzer_data.fuzz_args.compound_rewards_prob),
        ];

        for block_nonce in 1..=fuzzer_data.fuzz_args.num_events {
            let choice_index = WeightedIndex::new(choices.iter().map(|choice| choice.1)).unwrap();
            let random_choice = choices[choice_index.sample(&mut rng)].0;

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
                    enter_farm(&mut fuzzer_data, &mut farmer_info);
                }
                5 => {
                    println!("Event no. {}: Exit farm", (block_nonce));
                    exit_farm(&mut fuzzer_data, &mut farmer_info);
                }
                6 => {
                    println!("Event no. {}: Claim reward", (block_nonce));
                    claim_rewards(&mut fuzzer_data, &mut farmer_info);
                }
                7 => {
                    println!("Event no. {}: Compound reward", (block_nonce));
                    compound_rewards(&mut fuzzer_data, &mut farmer_info);
                }
                _ => println!("No event triggered"),
            }

            fuzzer_data.blockchain_wrapper.set_block_nonce(block_nonce);
        }

        print_statistics(&mut fuzzer_data);
    }

    fn print_statistics<PairObjBuilder, FarmObjBuilder>(
        fuzzer_data: &mut FuzzerData<PairObjBuilder, FarmObjBuilder>,
    ) where
        PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        println!();
        println!("Statistics:");
        println!(
            "Total number of events: {}",
            fuzzer_data.fuzz_args.num_events
        );
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
        println!(
            "addLiquidityPriceChecks: {}",
            fuzzer_data.statistics.add_liquidity_price_checks
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
        println!(
            "removeLiquidityPriceChecks: {}",
            fuzzer_data.statistics.remove_liquidity_price_checks
        );
        println!();
        println!(
            "enterFarmHits: {}",
            fuzzer_data.statistics.enter_farm_hits
        );
        println!(
            "enterFarmMisses: {}",
            fuzzer_data.statistics.enter_farm_misses
        );
        println!();
        println!(
            "exitFarmHits: {}",
            fuzzer_data.statistics.exit_farm_hits
        );
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
    }
}
