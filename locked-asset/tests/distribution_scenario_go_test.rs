use multiversx_sc_scenario::*;

fn world() -> ScenarioWorld {
    ScenarioWorld::vm_go()
}

#[test]
#[ignore]
fn add_liquidity_proxy_go() {
    world().run("scenarios/add_liquidity_proxy.scen.json");
}

#[test]
#[ignore]
fn add_liquidity_with_merge_tokens_go() {
    world().run("scenarios/add_liquidity_with_merge_tokens.scen.json");
}

#[test]
#[ignore]
fn claim_locked_assets_basic_go() {
    world().run("scenarios/claim_locked_assets_basic.scen.json");
}

#[test]
#[ignore]
fn claim_mex_rewards_proxy_after_mint_rewards_go() {
    world().run("scenarios/claim_mex_rewards_proxy_after_mint_rewards.scen.json");
}

#[test]
#[ignore]
fn claim_only_last_four_go() {
    world().run("scenarios/claim_only_last_four.scen.json");
}

#[test]
#[ignore]
fn claim_rewards_proxy_go() {
    world().run("scenarios/claim_rewards_proxy.scen.json");
}

#[test]
#[ignore]
fn clear_unclaimable_assets_go() {
    world().run("scenarios/clear_unclaimable_assets.scen.json");
}

#[test]
#[ignore]
fn compound_mex_rewards_proxy_after_mint_rewards_go() {
    world().run("scenarios/compound_mex_rewards_proxy_after_mint_rewards.scen.json");
}

#[test]
#[ignore]
fn enter_farm_proxy_go() {
    world().run("scenarios/enter_farm_proxy.scen.json");
}

#[test]
#[ignore]
fn enter_farm_proxy_with_merge_tokens_go() {
    world().run("scenarios/enter_farm_proxy_with_merge_tokens.scen.json");
}

#[test]
#[ignore]
fn enter_mex_farm_proxy_go() {
    world().run("scenarios/enter_mex_farm_proxy.scen.json");
}

#[test]
#[ignore]
fn exit_farm_proxy_go() {
    world().run("scenarios/exit_farm_proxy.scen.json");
}

#[test]
#[ignore]
fn exit_mex_farm_proxy_after_compound_rewards_go() {
    world().run("scenarios/exit_mex_farm_proxy_after_compound_rewards.scen.json");
}

#[test]
#[ignore]
fn exit_mex_farm_proxy_after_compound_rewards_and_epoch_increase_go() {
    world().run("scenarios/exit_mex_farm_proxy_after_compound_rewards_and_epoch_increase.scen.json");
}

#[test]
#[ignore]
fn exit_mex_farm_proxy_after_mint_rewards_go() {
    world().run("scenarios/exit_mex_farm_proxy_after_mint_rewards.scen.json");
}

#[test]
#[ignore]
fn merge_locked_mex_tokens_go() {
    world().run("scenarios/merge_locked_mex_tokens.scen.json");
}

#[test]
#[ignore]
fn merge_wrapped_farm_tokens_go() {
    world().run("scenarios/merge_wrapped_farm_tokens.scen.json");
}

#[test]
#[ignore]
fn merge_wrapped_lp_tokens_go() {
    world().run("scenarios/merge_wrapped_lp_tokens.scen.json");
}

#[test]
#[ignore]
fn multiple_claim_assets_go() {
    world().run("scenarios/multiple_claim_assets.scen.json");
}

#[test]
#[ignore]
fn remove_liquidity_proxy_go() {
    world().run("scenarios/remove_liquidity_proxy.scen.json");
}

#[test]
#[ignore]
fn remove_liquidity_proxy_after_swap_mex_go() {
    world().run("scenarios/remove_liquidity_proxy_after_swap_mex.scen.json");
}

#[test]
#[ignore]
fn remove_liquidity_proxy_after_swap_wegld_go() {
    world().run("scenarios/remove_liquidity_proxy_after_swap_wegld.scen.json");
}

#[test]
#[ignore]
fn set_user_distribution_go() {
    world().run("scenarios/set_user_distribution.scen.json");
}

#[test]
#[ignore]
fn set_user_distribution_above_cap_go() {
    world().run("scenarios/set_user_distribution_above_cap.scen.json");
}

#[test]
#[ignore]
fn set_user_distribution_duplicate_go() {
    world().run("scenarios/set_user_distribution_duplicate.scen.json");
}

#[test]
#[ignore]
fn set_user_distribution_with_unlock_go() {
    world().run("scenarios/set_user_distribution_with_unlock.scen.json");
}

#[test]
#[ignore]
fn undo_last_community_distribution_go() {
    world().run("scenarios/undo_last_community_distribution.scen.json");
}

#[test]
#[ignore]
fn undo_user_distribution_between_epochs_go() {
    world().run("scenarios/undo_user_distribution_between_epochs.scen.json");
}

#[test]
#[ignore]
fn unlock_assets_basic_go() {
    world().run("scenarios/unlock_assets_basic.scen.json");
}
