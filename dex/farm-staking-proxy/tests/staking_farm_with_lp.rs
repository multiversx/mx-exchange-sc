use elrond_wasm::types::Address;
use elrond_wasm_debug::{
    rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};

pub mod constants;
pub mod staking_farm_with_lp_external_contracts;
pub mod staking_farm_with_lp_staking_contract_interactions;
pub mod staking_farm_with_lp_staking_contract_setup;

use constants::*;
use staking_farm_with_lp_external_contracts::*;
use staking_farm_with_lp_staking_contract_interactions::*;
use staking_farm_with_lp_staking_contract_setup::*;

#[test]
fn test_all_setup() {
    let _ = FarmStakingSetup::new(
        pair::contract_obj,
        farm::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );
}

#[test]
fn test_stake_farm_proxy() {
    let mut setup = FarmStakingSetup::new(
        pair::contract_obj,
        farm::contract_obj,
        farm_staking::contract_obj,
        farm_staking_proxy::contract_obj,
    );

    let expected_staking_token_amount = 166_666; // safe price of USER_TOTAL_LP_TOKENS in RIDE tokens
    let _dual_yield_token_nonce =
        setup.stake_farm_lp(1, USER_TOTAL_LP_TOKENS, 1, expected_staking_token_amount);
}
