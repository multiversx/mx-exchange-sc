use multiversx_sc_scenario::*;

fn world() -> ScenarioWorld {
    ScenarioWorld::vm_go()
}

#[test]
fn add_liquidity_go() {
    world().run("scenarios/add_liquidity.scen.json");
}

#[test]
fn calculate_rewards_for_given_position_go() {
    world().run("scenarios/calculate_rewards_for_given_position.scen.json");
}

#[test]
fn calculate_rewards_for_given_position_after_compound_go() {
    world().run("scenarios/calculate_rewards_for_given_position_after_compound.scen.json");
}

#[test]
fn check_fee_disabled_after_swap_go() {
    world().run("scenarios/check_fee_disabled_after_swap.scen.json");
}

#[test]
fn check_fee_enabled_after_swap_go() {
    world().run("scenarios/check_fee_enabled_after_swap.scen.json");
}

#[test]
fn claim_rewards_go() {
    world().run("scenarios/claim_rewards.scen.json");
}

#[test]
fn complete_setup_go() {
    world().run("scenarios/complete_setup.scen.json");
}

#[test]
fn compound_rewards_go() {
    world().run("scenarios/compound_rewards.scen.json");
}

#[test]
fn create_pair_twice_go() {
    world().run("scenarios/create_pair_twice.scen.json");
}

#[test]
fn enter_farm_go() {
    world().run("scenarios/enter_farm.scen.json");
}

#[test]
fn enter_farm_with_merge_tokens_go() {
    world().run("scenarios/enter_farm_with_merge_tokens.scen.json");
}

#[test]
fn enter_mex_farm_go() {
    world().run("scenarios/enter_mex_farm.scen.json");
}

#[test]
fn exit_farm_go() {
    world().run("scenarios/exit_farm.scen.json");
}

#[test]
fn exit_farm_too_soon_go() {
    world().run("scenarios/exit_farm_too_soon.scen.json");
}

#[test]
fn exit_mex_farm_go() {
    world().run("scenarios/exit_mex_farm.scen.json");
}

#[test]
fn farm_reward_distr_scen_1_go() {
    world().run("scenarios/farm_reward_distr_scen_1.scen.json");
}

#[test]
fn farm_reward_distr_scen_2_go() {
    world().run("scenarios/farm_reward_distr_scen_2.scen.json");
}

#[test]
fn farm_reward_distr_scen_3_go() {
    world().run("scenarios/farm_reward_distr_scen_3.scen.json");
}

#[test]
fn farm_reward_distr_scen_4_go() {
    world().run("scenarios/farm_reward_distr_scen_4.scen.json");
}

#[test]
fn farm_with_egld_token_go() {
    world().run("scenarios/farm_with_egld_token.scen.json");
}

#[test]
fn farm_wrong_lp_token_go() {
    world().run("scenarios/farm_wrong_lp_token.scen.json");
}

#[test]
fn get_amounts_go() {
    world().run("scenarios/get_amounts.scen.json");
}

#[test]
fn get_amounts_no_liquidity_go() {
    world().run("scenarios/get_amounts_no_liquidity.scen.json");
}

#[test]
fn get_pair_non_existent_go() {
    world().run("scenarios/get_pair_non_existent.scen.json");
}

#[test]
fn get_pair_views_go() {
    world().run("scenarios/get_pair_views.scen.json");
}

#[test]
fn merge_tokens_go() {
    world().run("scenarios/merge_tokens.scen.json");
}

#[test]
fn owner_pause_farm_go() {
    world().run("scenarios/owner_pause_farm.scen.json");
}

#[test]
fn owner_resume_farm_go() {
    world().run("scenarios/owner_resume_farm.scen.json");
}

#[test]
fn remove_liquidity_go() {
    world().run("scenarios/remove_liquidity.scen.json");
}

#[test]
fn remove_liquidity_and_buyback_and_burn_token_go() {
    world().run("scenarios/remove_liquidity_and_buyback_and_burn_token.scen.json");
}

#[test]
fn remove_liquidity_twice_go() {
    world().run("scenarios/remove_liquidity_twice.scen.json");
}

#[test]
fn remove_pair_go() {
    world().run("scenarios/remove_pair.scen.json");
}

#[test]
fn router_pause_self_go() {
    world().run("scenarios/router_pause_self.scen.json");
}

#[test]
fn router_resume_self_go() {
    world().run("scenarios/router_resume_self.scen.json");
}

#[test]
fn swap_fixed_input_go() {
    world().run("scenarios/swap_fixed_input.scen.json");
}

#[test]
fn swap_fixed_input_after_removed_liquidity_go() {
    world().run("scenarios/swap_fixed_input_after_removed_liquidity.scen.json");
}

#[test]
fn swap_fixed_output_go() {
    world().run("scenarios/swap_fixed_output.scen.json");
}

#[test]
fn swap_same_token_go() {
    world().run("scenarios/swap_same_token.scen.json");
}

#[test]
fn swap_wrong_token_go() {
    world().run("scenarios/swap_wrong_token.scen.json");
}

#[test]
fn upgrade_contract_go() {
    world().run("scenarios/upgrade_contract.scen.json");
}
