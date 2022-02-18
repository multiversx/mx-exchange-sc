
#[cfg(test)]
mod test {

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::fuzz_farm::fuzz_farm_test::*;
use crate::fuzz_pair::fuzz_pair_test::*;
use crate::fuzz_data::fuzz_data_tests::*;

use elrond_wasm_debug::DebugApi;

use rand::Rng;

    #[test]
    fn start_fuzzer() {
        let mut fuzzer_data = FuzzerData::new(pair::contract_obj, farm::contract_obj);

        for block_nonce in 1..=fuzzer_data.fuzz_args.num_events {
            let mut rng = rand::thread_rng();
            let random_choice = rng.gen_range(1..=5);

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
        println!("");
        println!("Statistics:");
        println!(
            "Total number of events: {}",
            fuzzer_data.fuzz_args.num_events
        );
        println!("");
        println!(
            "swapFixedInputHits: {}",
            fuzzer_data.statistics.swap_fixed_input_hits.get()
        );
        println!(
            "swapFixedInputMisses: {}",
            fuzzer_data.statistics.swap_fixed_input_misses.get()
        );
        println!("");
        println!(
            "swapFixedOutputHits: {}",
            fuzzer_data.statistics.swap_fixed_output_hits.get()
        );
        println!(
            "swapFixedOutputMissed: {}",
            fuzzer_data.statistics.swap_fixed_output_misses.get()
        );
        println!("");
        println!(
            "addLiquidityHits: {}",
            fuzzer_data.statistics.add_liquidity_hits.get()
        );
        println!(
            "addLiquidityMisses: {}",
            fuzzer_data.statistics.add_liquidity_misses.get()
        );
        println!(
            "addLiquidityPriceChecks: {}",
            fuzzer_data.statistics.add_liquidity_price_checks.get()
        );
        println!("");
        println!(
            "removeLiquidityHits: {}",
            fuzzer_data.statistics.remove_liquidity_hits.get()
        );
        println!(
            "removeLiquidityMisses: {}",
            fuzzer_data.statistics.remove_liquidity_misses.get()
        );
        println!(
            "removeLiquidityPriceChecks: {}",
            fuzzer_data.statistics.remove_liquidity_price_checks.get()
        );
        println!("");
        println!(
            "queryPairHits: {}",
            fuzzer_data.statistics.query_pairs_hits.get()
        );
        println!(
            "queryPairMisses: {}",
            fuzzer_data.statistics.query_pairs_misses.get()
        );
        println!("");
        println!(
            "enterFarmHits: {}",
            fuzzer_data.statistics.enter_farm_hits.get()
        );
        println!(
            "enterFarmMisses: {}",
            fuzzer_data.statistics.enter_farm_misses.get()
        );
        println!("");
        println!(
            "exitFarmHits: {}",
            fuzzer_data.statistics.exit_farm_hits.get()
        );
        println!(
            "exitFarmMisses: {}",
            fuzzer_data.statistics.exit_farm_misses.get()
        );
        println!(
            "exitFarmWithRewards: {}",
            fuzzer_data.statistics.exit_farm_with_rewards.get()
        );
        println!("");
        println!(
            "claimRewardsHits: {}",
            fuzzer_data.statistics.claim_rewards_hits.get()
        );
        println!(
            "claimRewardsMisses: {}",
            fuzzer_data.statistics.claim_rewards_misses.get()
        );
        println!(
            "claimRewardsWithRewards: {}",
            fuzzer_data.statistics.claim_rewards_with_rewards.get()
        );
        println!("");
        println!(
            "compoundRewardsHits: {}",
            fuzzer_data.statistics.compound_rewards_hits.get()
        );
        println!(
            "compoundRewardsMisses: {}",
            fuzzer_data.statistics.compound_rewards_misses.get()
        );
        println!("");
    }
}
