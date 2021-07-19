#[test]
fn accept_esdt_payment_go() {
    elrond_wasm_debug::mandos_go("mandos/accept_esdt_payment.scen.json");
}

#[test]
fn accept_esdt_payment_wrong_token_go() {
    elrond_wasm_debug::mandos_go("mandos/accept_esdt_payment_wrong_token.scen.json");
}

#[test]
fn add_liquidity_go() {
    elrond_wasm_debug::mandos_go("mandos/add_liquidity.scen.json");
}

#[test]
fn calculate_rewards_for_given_position_go() {
    elrond_wasm_debug::mandos_go("mandos/calculate_rewards_for_given_position.scen.json");
}

#[test]
fn check_fee_disabled_after_swap_go() {
    elrond_wasm_debug::mandos_go("mandos/check_fee_disabled_after_swap.scen.json");
}

#[test]
fn check_fee_enabled_after_swap_go() {
    elrond_wasm_debug::mandos_go("mandos/check_fee_enabled_after_swap.scen.json");
}

#[test]
fn claim_rewards_go() {
    elrond_wasm_debug::mandos_go("mandos/claim_rewards.scen.json");
}

#[test]
fn complete_setup_go() {
    elrond_wasm_debug::mandos_go("mandos/complete_setup.scen.json");
}

#[test]
fn create_pair_twice_go() {
    elrond_wasm_debug::mandos_go("mandos/create_pair_twice.scen.json");
}

#[test]
fn enter_farm_go() {
    elrond_wasm_debug::mandos_go("mandos/enter_farm.scen.json");
}

#[test]
fn enter_mex_farm_go() {
    elrond_wasm_debug::mandos_go("mandos/enter_mex_farm.scen.json");
}

#[test]
fn exit_farm_go() {
    elrond_wasm_debug::mandos_go("mandos/exit_farm.scen.json");
}

#[test]
fn exit_farm_too_soon_go() {
    elrond_wasm_debug::mandos_go("mandos/exit_farm_too_soon.scen.json");
}

#[test]
fn exit_mex_farm_go() {
    elrond_wasm_debug::mandos_go("mandos/exit_mex_farm.scen.json");
}

#[test]
fn farm_reward_distr_scen_1_go() {
    elrond_wasm_debug::mandos_go("mandos/farm_reward_distr_scen_1.scen.json");
}

#[test]
fn farm_reward_distr_scen_2_go() {
    elrond_wasm_debug::mandos_go("mandos/farm_reward_distr_scen_2.scen.json");
}

#[test]
fn farm_reward_distr_scen_3_go() {
    elrond_wasm_debug::mandos_go("mandos/farm_reward_distr_scen_3.scen.json");
}

#[test]
fn farm_reward_distr_scen_4_go() {
    elrond_wasm_debug::mandos_go("mandos/farm_reward_distr_scen_4.scen.json");
}

#[test]
fn farm_reward_distr_scen_5_go() {
    elrond_wasm_debug::mandos_go("mandos/farm_reward_distr_scen_5.scen.json");
}

#[test]
fn farm_with_egld_token_go() {
    elrond_wasm_debug::mandos_go("mandos/farm_with_egld_token.scen.json");
}

#[test]
fn farm_wrong_lp_token_go() {
    elrond_wasm_debug::mandos_go("mandos/farm_wrong_lp_token.scen.json");
}

#[test]
fn get_amounts_go() {
    elrond_wasm_debug::mandos_go("mandos/get_amounts.scen.json");
}

#[test]
fn get_amounts_no_liquidity_go() {
    elrond_wasm_debug::mandos_go("mandos/get_amounts_no_liquidity.scen.json");
}

#[test]
fn get_pair_non_existent_go() {
    elrond_wasm_debug::mandos_go("mandos/get_pair_non_existent.scen.json");
}

#[test]
fn get_pair_views_go() {
    elrond_wasm_debug::mandos_go("mandos/get_pair_views.scen.json");
}

#[test]
fn owner_pause_farm_go() {
    elrond_wasm_debug::mandos_go("mandos/owner_pause_farm.scen.json");
}

#[test]
fn owner_resume_farm_go() {
    elrond_wasm_debug::mandos_go("mandos/owner_resume_farm.scen.json");
}

#[test]
fn reclaim_temporary_funds_go() {
    elrond_wasm_debug::mandos_go("mandos/reclaim_temporary_funds.scen.json");
}

#[test]
fn remove_liquidity_go() {
    elrond_wasm_debug::mandos_go("mandos/remove_liquidity.scen.json");
}

#[test]
fn remove_liquidity_twice_go() {
    elrond_wasm_debug::mandos_go("mandos/remove_liquidity_twice.scen.json");
}

#[test]
fn router_pause_self_go() {
    elrond_wasm_debug::mandos_go("mandos/router_pause_self.scen.json");
}

#[test]
fn router_resume_self_go() {
    elrond_wasm_debug::mandos_go("mandos/router_resume_self.scen.json");
}

#[test]
fn send_with_no_funds_go() {
    elrond_wasm_debug::mandos_go("mandos/send_with_no_funds.scen.json");
}

#[test]
fn swap_fixed_input_go() {
    elrond_wasm_debug::mandos_go("mandos/swap_fixed_input.scen.json");
}

#[test]
fn swap_fixed_input_after_removed_liquidity_go() {
    elrond_wasm_debug::mandos_go("mandos/swap_fixed_input_after_removed_liquidity.scen.json");
}

#[test]
fn swap_fixed_output_go() {
    elrond_wasm_debug::mandos_go("mandos/swap_fixed_output.scen.json");
}

#[test]
fn swap_same_token_go() {
    elrond_wasm_debug::mandos_go("mandos/swap_same_token.scen.json");
}

#[test]
fn swap_wrong_token_go() {
    elrond_wasm_debug::mandos_go("mandos/swap_wrong_token.scen.json");
}

#[test]
fn upgrade_contract_go() {
    elrond_wasm_debug::mandos_go("mandos/upgrade_contract.scen.json");
}
