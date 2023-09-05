#[cfg(test)]
pub mod fuzz_price_discovery_test {
    #![allow(deprecated)]

    multiversx_sc::imports!();
    multiversx_sc::derive_imports!();

    use multiversx_sc_scenario::{rust_biguint, DebugApi};

    use rand::prelude::*;

    use crate::fuzz_data::fuzz_data_tests::*;
    use price_discovery::PriceDiscovery;

    pub fn price_discovery_deposit<
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
        let caller_index = fuzzer_data.rng.gen_range(0..fuzzer_data.users.len());
        let caller = &mut fuzzer_data.users[caller_index];
        let price_disc = &mut fuzzer_data.price_disc;

        let redeem_token_id = DISC_REDEEM_TOKEN_ID;
        let (payment_token_id, redeem_token_nonce) = if caller.price_discovery_buy {
            (DISC_ACCEPTED_TOKEN_ID, DISC_ACCEPTED_TOKEN_REDEEM_NONCE)
        } else {
            (DISC_LAUNCHED_TOKEN_ID, DISC_LAUNCHED_TOKEN_REDEEM_NONCE)
        };

        let seed = fuzzer_data
            .rng
            .gen_range(0..fuzzer_data.fuzz_args.price_discovery_deposit_max_value)
            + 1;

        let deposit_amount = rust_biguint!(seed);

        let caller_token_before =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, payment_token_id, 0);

        let redeem_token_before = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            &caller.address,
            redeem_token_id,
            redeem_token_nonce,
        );

        let sc_launched_token_before = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            price_disc.pd_wrapper.address_ref(),
            DISC_LAUNCHED_TOKEN_ID,
            0,
        );

        let sc_accepted_token_before = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            price_disc.pd_wrapper.address_ref(),
            DISC_ACCEPTED_TOKEN_ID,
            0,
        );

        if caller_token_before < deposit_amount {
            println!("Price discovery deposit error: Not enough tokens");
            fuzzer_data.statistics.price_discovery_deposit_misses += 1;

            return;
        }

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_transfer(
            &caller.address,
            &price_disc.pd_wrapper,
            payment_token_id,
            0,
            &deposit_amount,
            |sc| {
                let _ = sc.deposit();
            },
        );

        let caller_token_after =
            fuzzer_data
                .blockchain_wrapper
                .get_esdt_balance(&caller.address, payment_token_id, 0);

        let redeem_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            &caller.address,
            redeem_token_id,
            redeem_token_nonce,
        );

        let sc_launched_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            price_disc.pd_wrapper.address_ref(),
            DISC_LAUNCHED_TOKEN_ID,
            0,
        );

        let sc_accepted_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            price_disc.pd_wrapper.address_ref(),
            DISC_ACCEPTED_TOKEN_ID,
            0,
        );

        let tx_result_string = tx_result.result_message;
        let tx_result_msg_is_empty = tx_result_string.trim().is_empty();

        if caller.price_discovery_buy && tx_result_msg_is_empty {
            if sc_accepted_token_after != sc_accepted_token_before + &deposit_amount {
                println!(
                    "Price discovery deposit error: sc accepted token final balance is incorrect"
                );
                fuzzer_data.statistics.price_discovery_deposit_misses += 1;
                return;
            }
        } else if tx_result_msg_is_empty
            && sc_launched_token_after != sc_launched_token_before + &deposit_amount
        {
            println!("Price discovery deposit error: sc launched token final balance is incorrect");
            fuzzer_data.statistics.price_discovery_deposit_misses += 1;
            return;
        }

        if !tx_result_msg_is_empty {
            println!("Price discovery deposit error: {}", tx_result_string);
            fuzzer_data.statistics.price_discovery_deposit_misses += 1;
        } else if caller_token_after != caller_token_before - &deposit_amount {
            println!("Price discovery deposit error: deposit token final balance is incorrect");
            fuzzer_data.statistics.price_discovery_deposit_misses += 1;
        } else if redeem_token_after != redeem_token_before + &deposit_amount {
            println!("Price discovery deposit error: redeem token final balance is incorrect");
            fuzzer_data.statistics.price_discovery_deposit_misses += 1;
        } else if tx_result_msg_is_empty {
            fuzzer_data.statistics.price_discovery_deposit_hits += 1;
        } else {
            println!("!!! Price discovery withdraw error: undefined case");
            fuzzer_data.statistics.price_discovery_deposit_misses += 1;
        }
    }

    pub fn price_discovery_withdraw<
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
        let caller_index = fuzzer_data.rng.gen_range(0..fuzzer_data.users.len());
        let caller = &mut fuzzer_data.users[caller_index];
        let price_disc = &mut fuzzer_data.price_disc;

        let redeem_token_id = DISC_REDEEM_TOKEN_ID;
        let redeem_token_nonce = if caller.price_discovery_buy {
            DISC_ACCEPTED_TOKEN_REDEEM_NONCE
        } else {
            DISC_LAUNCHED_TOKEN_REDEEM_NONCE
        };

        let seed = fuzzer_data
            .rng
            .gen_range(0..fuzzer_data.fuzz_args.price_discovery_withdraw_max_value)
            + 1;

        let redeem_token_in_amount = rust_biguint!(seed);

        let sc_launched_token_before = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            price_disc.pd_wrapper.address_ref(),
            DISC_LAUNCHED_TOKEN_ID,
            0,
        );

        let sc_accepted_token_before = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            price_disc.pd_wrapper.address_ref(),
            DISC_ACCEPTED_TOKEN_ID,
            0,
        );

        let redeem_token_before = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            &caller.address,
            redeem_token_id,
            redeem_token_nonce,
        );

        if redeem_token_before < redeem_token_in_amount {
            println!("Price discovery withdraw error: Not enough redeem tokens");
            fuzzer_data.statistics.price_discovery_withdraw_misses += 1;

            return;
        }

        let mut withdrawn_amount = rust_biguint!(0);
        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_transfer(
            &caller.address,
            &price_disc.pd_wrapper,
            redeem_token_id,
            redeem_token_nonce,
            &redeem_token_in_amount,
            |sc| {
                let output_payment = sc.withdraw();
                withdrawn_amount = num_bigint::BigUint::from_bytes_be(
                    output_payment.amount.to_bytes_be().as_slice(),
                );
            },
        );

        let redeem_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            &caller.address,
            redeem_token_id,
            redeem_token_nonce,
        );

        let sc_launched_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            price_disc.pd_wrapper.address_ref(),
            DISC_LAUNCHED_TOKEN_ID,
            0,
        );

        let sc_accepted_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            price_disc.pd_wrapper.address_ref(),
            DISC_ACCEPTED_TOKEN_ID,
            0,
        );

        let tx_result_string = tx_result.result_message;
        let tx_result_msg_is_empty = tx_result_string.trim().is_empty();

        if caller.price_discovery_buy && tx_result_msg_is_empty {
            if sc_accepted_token_after != sc_accepted_token_before - &withdrawn_amount {
                println!(
                    "Price discovery withdraw error: sc accepted token final balance is incorrect"
                );
                fuzzer_data.statistics.price_discovery_withdraw_misses += 1;
                return;
            }
        } else if tx_result_msg_is_empty
            && sc_launched_token_after != sc_launched_token_before - &withdrawn_amount
        {
            println!(
                "Price discovery withdraw error: sc launched token final balance is incorrect"
            );
            fuzzer_data.statistics.price_discovery_withdraw_misses += 1;
            return;
        }

        if !tx_result_msg_is_empty {
            println!("Price discovery withdraw error: {}", tx_result_string);
            fuzzer_data.statistics.price_discovery_withdraw_misses += 1;
        } else if redeem_token_after != redeem_token_before - &redeem_token_in_amount {
            println!("Price discovery withdraw error: redeem token final balance is incorrect");
            fuzzer_data.statistics.price_discovery_withdraw_misses += 1;
        } else if tx_result_msg_is_empty {
            fuzzer_data.statistics.price_discovery_withdraw_hits += 1;
        } else {
            println!("!!! Price discovery withdraw error: undefined case");
            fuzzer_data.statistics.price_discovery_withdraw_misses += 1;
        }
    }

    pub fn price_discovery_redeem<
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

        let caller_index = fuzzer_data.rng.gen_range(0..fuzzer_data.users.len());
        let caller = &mut fuzzer_data.users[caller_index];
        let price_disc = &mut fuzzer_data.price_disc;

        let redeem_token_id = DISC_REDEEM_TOKEN_ID;
        let redeem_token_nonce = if caller.price_discovery_buy {
            DISC_ACCEPTED_TOKEN_REDEEM_NONCE
        } else {
            DISC_LAUNCHED_TOKEN_REDEEM_NONCE
        };

        let seed = fuzzer_data
            .rng
            .gen_range(0..fuzzer_data.fuzz_args.price_discovery_redeem_max_value)
            + 1;

        let mut redeem_token_amount_in = rust_biguint!(seed);

        let redeem_token_before = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            &caller.address,
            redeem_token_id,
            redeem_token_nonce,
        );

        if redeem_token_before < redeem_token_amount_in {
            if redeem_token_amount_in > rust_zero {
                redeem_token_amount_in = redeem_token_before.clone();
            } else {
                println!("Price discovery redeem error: Not enough tokens");
                fuzzer_data.statistics.price_discovery_redeem_misses += 1;

                return;
            }
        }

        let tx_result = fuzzer_data.blockchain_wrapper.execute_esdt_transfer(
            &caller.address,
            &price_disc.pd_wrapper,
            redeem_token_id,
            redeem_token_nonce,
            &redeem_token_amount_in,
            |sc| {
                let _ = sc.redeem();
            },
        );

        let redeem_token_after = fuzzer_data.blockchain_wrapper.get_esdt_balance(
            &caller.address,
            redeem_token_id,
            redeem_token_nonce,
        );

        let tx_result_string = tx_result.result_message;
        let tx_result_msg_is_empty = tx_result_string.trim().is_empty();

        if !tx_result_msg_is_empty {
            println!("Price discovery redeem error: {}", tx_result_string);
            fuzzer_data.statistics.price_discovery_redeem_misses += 1;
            return;
        }

        if redeem_token_after != redeem_token_before - &redeem_token_amount_in {
            println!("Price discovery redeem error: redeem token final balance is incorrect");
            fuzzer_data.statistics.price_discovery_redeem_misses += 1;
        } else {
            fuzzer_data.statistics.price_discovery_redeem_hits += 1;
        }
    }
}
