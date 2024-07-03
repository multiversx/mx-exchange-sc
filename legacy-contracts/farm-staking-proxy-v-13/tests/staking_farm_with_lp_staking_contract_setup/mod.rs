// use external_contracts_interactions::ExternalContractsInteractionsModule;
// use farm_token::FarmTokenModule;
// use lp_farm_token::LpFarmTokenModule;
// use multiversx_sc::{
//     imports::StorageTokenWrapper,
//     types::{Address, EsdtLocalRole, MultiValueEncoded},
// };
// use multiversx_sc_scenario::{
//     managed_address, managed_biguint, managed_token_id, rust_biguint,
//     testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
//     DebugApi,
// };

// use farm_staking::*;

// use farm_staking_proxy_legacy::dual_yield_token::DualYieldTokenModule;
// use farm_staking_proxy_legacy::*;
// use pausable::{PausableModule, State};
// use sc_whitelist_module::SCWhitelistModule;

// use crate::constants::*;

// pub fn setup_staking_farm<StakingContractObjBuilder>(
//     owner_addr: &Address,
//     b_mock: &mut BlockchainStateWrapper,
//     builder: StakingContractObjBuilder,
// ) -> ContractObjWrapper<farm_staking::ContractObj<DebugApi>, StakingContractObjBuilder>
// where
//     StakingContractObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
// {
//     let rust_zero = rust_biguint!(0u64);
//     let farm_staking_wrapper =
//         b_mock.create_sc_account(&rust_zero, Some(owner_addr), builder, PROXY_WASM_PATH);

//     b_mock
//         .execute_tx(owner_addr, &farm_staking_wrapper, &rust_zero, |sc| {
//             let farming_token_id = managed_token_id!(STAKING_TOKEN_ID);
//             let div_const = managed_biguint!(DIVISION_SAFETY_CONSTANT);
//             let max_apr = managed_biguint!(MAX_APR);

//             sc.init(
//                 farming_token_id,
//                 div_const,
//                 max_apr,
//                 UNBOND_EPOCHS,
//                 managed_address!(owner_addr),
//                 MultiValueEncoded::new(),
//             );

//             sc.farm_token()
//                 .set_token_id(managed_token_id!(STAKING_FARM_TOKEN_ID));

//             sc.state().set(State::Active);
//         })
//         .assert_ok();

//     b_mock.set_esdt_balance(
//         farm_staking_wrapper.address_ref(),
//         STAKING_REWARD_TOKEN_ID,
//         &rust_biguint!(REWARD_CAPACITY),
//     );

//     let farm_token_roles = [
//         EsdtLocalRole::NftCreate,
//         EsdtLocalRole::NftAddQuantity,
//         EsdtLocalRole::NftBurn,
//     ];
//     b_mock.set_esdt_local_roles(
//         farm_staking_wrapper.address_ref(),
//         STAKING_FARM_TOKEN_ID,
//         &farm_token_roles[..],
//     );

//     farm_staking_wrapper
// }

// pub fn add_proxy_to_whitelist<FarmObjBuilder, StakingContractObjBuilder>(
//     owner_addr: &Address,
//     proxy_address: &Address,
//     b_mock: &mut BlockchainStateWrapper,
//     lp_farm_builder: &ContractObjWrapper<
//         farm_with_locked_rewards::ContractObj<DebugApi>,
//         FarmObjBuilder,
//     >,
//     staking_farm_builder: &ContractObjWrapper<
//         farm_staking::ContractObj<DebugApi>,
//         StakingContractObjBuilder,
//     >,
// ) where
//     FarmObjBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
//     StakingContractObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
// {
//     let rust_zero = rust_biguint!(0u64);
//     b_mock
//         .execute_tx(owner_addr, lp_farm_builder, &rust_zero, |sc| {
//             sc.add_sc_address_to_whitelist(managed_address!(proxy_address));
//         })
//         .assert_ok();
//     b_mock
//         .execute_tx(owner_addr, staking_farm_builder, &rust_zero, |sc| {
//             sc.add_sc_address_to_whitelist(managed_address!(proxy_address));
//         })
//         .assert_ok();
// }

// pub fn setup_proxy<ProxyContractObjBuilder>(
//     owner_addr: &Address,
//     lp_farm_address: &Address,
//     staking_farm_address: &Address,
//     pair_address: &Address,
//     b_mock: &mut BlockchainStateWrapper,
//     builder: ProxyContractObjBuilder,
// ) -> ContractObjWrapper<farm_staking_proxy_legacy::ContractObj<DebugApi>, ProxyContractObjBuilder>
// where
//     ProxyContractObjBuilder:
//         'static + Copy + Fn() -> farm_staking_proxy_legacy::ContractObj<DebugApi>,
// {
//     let rust_zero = rust_biguint!(0u64);
//     let proxy_wrapper =
//         b_mock.create_sc_account(&rust_zero, Some(owner_addr), builder, PROXY_WASM_PATH);

//     b_mock
//         .execute_tx(owner_addr, &proxy_wrapper, &rust_zero, |sc| {
//             sc.init();

//             sc.pair_address().set(managed_address!(pair_address));
//             sc.lp_farm_address().set(managed_address!(lp_farm_address));
//             sc.lp_farm_token_id()
//                 .set(managed_token_id!(LP_FARM_TOKEN_ID));
//             sc.staking_farm_address()
//                 .set(managed_address!(staking_farm_address));
//             sc.staking_token_id()
//                 .set(managed_token_id!(STAKING_TOKEN_ID));
//             sc.staking_farm_token_id()
//                 .set(managed_token_id!(STAKING_FARM_TOKEN_ID));
//             sc.dual_yield_token_id()
//                 .set(&managed_token_id!(DUAL_YIELD_TOKEN_ID));
//         })
//         .assert_ok();

//     let dual_yield_token_roles = [
//         EsdtLocalRole::NftCreate,
//         EsdtLocalRole::NftAddQuantity,
//         EsdtLocalRole::NftBurn,
//     ];
//     b_mock.set_esdt_local_roles(
//         proxy_wrapper.address_ref(),
//         DUAL_YIELD_TOKEN_ID,
//         &dual_yield_token_roles[..],
//     );

//     proxy_wrapper
// }
