//

// pub mod constants;
// pub mod staking_farm_with_lp_external_contracts;
// pub mod staking_farm_with_lp_staking_contract_interactions;
// pub mod staking_farm_with_lp_staking_contract_setup;

// multiversx_sc::imports!();

// use common_structs::FarmTokenAttributes;
// use constants::*;
// use farm_staking_proxy_legacy::dual_yield_token::DualYieldTokenAttributes;

// use farm_staking::stake_farm::StakeFarmModule;
// use farm_with_locked_rewards::Farm;
// use multiversx_sc_scenario::{
//     imports::TxTokenTransfer, managed_address, managed_biguint, rust_biguint, DebugApi,
// };
// use pair::pair_actions::add_liq::AddLiquidityModule;
// use staking_farm_with_lp_staking_contract_interactions::*;

// #[test]
// fn test_all_setup() {
//     let _ = FarmStakingSetup::new(
//         pair::contract_obj,
//         farm_with_locked_rewards::contract_obj,
//         energy_factory::contract_obj,
//         farm_staking::contract_obj,
//         farm_staking_proxy_legacy::contract_obj,
//     );
// }

// #[test]
// fn test_unstake_from_legacy_proxy() {
//     let mut setup = FarmStakingSetup::new(
//         pair::contract_obj,
//         farm_with_locked_rewards::contract_obj,
//         energy_factory::contract_obj,
//         farm_staking::contract_obj,
//         farm_staking_proxy_legacy::contract_obj,
//     );

//     DebugApi::dummy();
//     setup
//         .b_mock
//         .set_block_nonce(BLOCK_NONCE_AFTER_PAIR_SETUP + 20);
//     setup.b_mock.set_block_epoch(20);

//     let token_amount = 1_000_000_000u64;

//     let payments = vec![
//         TxTokenTransfer {
//             token_identifier: WEGLD_TOKEN_ID.to_vec(),
//             nonce: 0,
//             value: rust_biguint!(token_amount),
//         },
//         TxTokenTransfer {
//             token_identifier: RIDE_TOKEN_ID.to_vec(),
//             nonce: 0,
//             value: rust_biguint!(token_amount),
//         },
//     ];
//     setup
//         .b_mock
//         .execute_esdt_multi_transfer(&setup.user_addr, &setup.pair_wrapper, &payments, |sc| {
//             sc.add_liquidity(managed_biguint!(1u64), managed_biguint!(1u64));
//         })
//         .assert_ok();

//     setup
//         .b_mock
//         .execute_esdt_transfer(
//             &setup.user_addr,
//             &setup.lp_farm_wrapper,
//             LP_TOKEN_ID,
//             0,
//             &rust_biguint!(token_amount),
//             |sc| {
//                 sc.enter_farm_endpoint(OptionalValue::None);
//             },
//         )
//         .assert_ok();

//     // Simulate enter proxy staking contract
//     let lp_farm_token_attributes: FarmTokenAttributes<DebugApi> = FarmTokenAttributes {
//         reward_per_share: managed_biguint!(0),
//         entering_epoch: 20,
//         compounded_reward: managed_biguint!(0),
//         current_farm_amount: managed_biguint!(token_amount),
//         original_owner: managed_address!(&setup.user_addr),
//     };
//     setup.b_mock.set_nft_balance(
//         setup.proxy_wrapper.address_ref(),
//         LP_FARM_TOKEN_ID,
//         1,
//         &rust_biguint!(token_amount),
//         &lp_farm_token_attributes,
//     );
//     setup.b_mock.set_esdt_balance(
//         setup.proxy_wrapper.address_ref(),
//         RIDE_TOKEN_ID,
//         &rust_biguint!(token_amount),
//     );

//     setup
//         .b_mock
//         .execute_tx(
//             setup.proxy_wrapper.address_ref(),
//             &setup.staking_farm_wrapper,
//             &rust_biguint!(0u64),
//             |sc| {
//                 sc.stake_farm_through_proxy(
//                     managed_biguint!(token_amount),
//                     managed_address!(&setup.user_addr),
//                 );
//             },
//         )
//         .assert_ok();

//     let dual_yield_token_amount = token_amount;
//     let dual_yield_token_attributes: DualYieldTokenAttributes<DebugApi> =
//         DualYieldTokenAttributes {
//             lp_farm_token_nonce: 1,
//             lp_farm_token_amount: managed_biguint!(dual_yield_token_amount),
//             staking_farm_token_nonce: 1,
//             staking_farm_token_amount: managed_biguint!(dual_yield_token_amount),
//         };
//     setup.b_mock.set_nft_balance(
//         &setup.user_addr,
//         DUAL_YIELD_TOKEN_ID,
//         1,
//         &rust_biguint!(dual_yield_token_amount),
//         &dual_yield_token_attributes,
//     );

//     let expected_token_amount = 990_000_000u64;
//     setup.unstake_proxy(
//         1,
//         dual_yield_token_amount,
//         expected_token_amount,
//         expected_token_amount,
//         30,
//     );

//     setup.b_mock.set_block_epoch(30);
//     setup.unbond_proxy(2, expected_token_amount, expected_token_amount);
// }
