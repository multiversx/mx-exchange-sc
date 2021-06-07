#[test]
fn accept_esdt_payment_proxy_go() {
    elrond_wasm_debug::mandos_go("mandos/accept_esdt_payment_proxy.scen.json");
}

#[test]
fn add_liquidity_proxy_go() {
    elrond_wasm_debug::mandos_go("mandos/add_liquidity_proxy.scen.json");
}

#[test]
fn claim_locked_assets_basic_go() {
    elrond_wasm_debug::mandos_go("mandos/claim_locked_assets_basic.scen.json");
}

#[test]
fn claim_mex_rewards_proxy_after_mint_rewards_go() {
    elrond_wasm_debug::mandos_go("mandos/claim_mex_rewards_proxy_after_mint_rewards.scen.json");
}

#[test]
fn claim_only_last_four_go() {
    elrond_wasm_debug::mandos_go("mandos/claim_only_last_four.scen.json");
}

#[test]
fn claim_rewards_proxy_go() {
    elrond_wasm_debug::mandos_go("mandos/claim_rewards_proxy.scen.json");
}

#[test]
fn claim_rewards_proxy_after_enter_with_lock_go() {
    elrond_wasm_debug::mandos_go("mandos/claim_rewards_proxy_after_enter_with_lock.scen.json");
}

#[test]
fn claim_rewards_proxy_after_enter_with_lock_after_mint_rewards_go() {
    elrond_wasm_debug::mandos_go(
        "mandos/claim_rewards_proxy_after_enter_with_lock_after_mint_rewards.scen.json",
    );
}

#[test]
fn claim_rewards_proxy_after_mint_rewards_go() {
    elrond_wasm_debug::mandos_go("mandos/claim_rewards_proxy_after_mint_rewards.scen.json");
}

#[test]
fn clear_unclaimable_assets_go() {
    elrond_wasm_debug::mandos_go("mandos/clear_unclaimable_assets.scen.json");
}

#[test]
fn enter_farm_and_lock_rewards_proxy_go() {
    elrond_wasm_debug::mandos_go("mandos/enter_farm_and_lock_rewards_proxy.scen.json");
}

#[test]
fn enter_farm_proxy_go() {
    elrond_wasm_debug::mandos_go("mandos/enter_farm_proxy.scen.json");
}

#[test]
fn enter_mex_farm_proxy_go() {
    elrond_wasm_debug::mandos_go("mandos/enter_mex_farm_proxy.scen.json");
}

#[test]
fn exit_farm_proxy_go() {
    elrond_wasm_debug::mandos_go("mandos/exit_farm_proxy.scen.json");
}

#[test]
fn exit_farm_proxy_after_mint_rewards_go() {
    elrond_wasm_debug::mandos_go("mandos/exit_farm_proxy_after_mint_rewards.scen.json");
}

#[test]
fn exit_farm_proxy_with_lock_rewards_go() {
    elrond_wasm_debug::mandos_go("mandos/exit_farm_proxy_with_lock_rewards.scen.json");
}

#[test]
fn exit_farm_proxy_with_lock_rewards_after_mint_rewards_go() {
    elrond_wasm_debug::mandos_go(
        "mandos/exit_farm_proxy_with_lock_rewards_after_mint_rewards.scen.json",
    );
}

#[test]
fn exit_mex_farm_proxy_after_mint_rewards_go() {
    elrond_wasm_debug::mandos_go("mandos/exit_mex_farm_proxy_after_mint_rewards.scen.json");
}

#[test]
fn multiple_claim_assets_go() {
    elrond_wasm_debug::mandos_go("mandos/multiple_claim_assets.scen.json");
}

#[test]
fn reclaim_temporary_funds_proxy_go() {
    elrond_wasm_debug::mandos_go("mandos/reclaim_temporary_funds_proxy.scen.json");
}

#[test]
fn remove_liquidity_proxy_go() {
    elrond_wasm_debug::mandos_go("mandos/remove_liquidity_proxy.scen.json");
}

#[test]
fn remove_liquidity_proxy_after_swap_mex_go() {
    elrond_wasm_debug::mandos_go("mandos/remove_liquidity_proxy_after_swap_mex.scen.json");
}

#[test]
fn remove_liquidity_proxy_after_swap_wegld_go() {
    elrond_wasm_debug::mandos_go("mandos/remove_liquidity_proxy_after_swap_wegld.scen.json");
}

#[test]
fn set_user_distribution_go() {
    elrond_wasm_debug::mandos_go("mandos/set_user_distribution.scen.json");
}

#[test]
fn set_user_distribution_above_cap_go() {
    elrond_wasm_debug::mandos_go("mandos/set_user_distribution_above_cap.scen.json");
}

#[test]
fn set_user_distribution_duplicate_go() {
    elrond_wasm_debug::mandos_go("mandos/set_user_distribution_duplicate.scen.json");
}

#[test]
fn set_user_distribution_with_unlock_go() {
    elrond_wasm_debug::mandos_go("mandos/set_user_distribution_with_unlock.scen.json");
}

#[test]
fn undo_last_community_distribution_go() {
    elrond_wasm_debug::mandos_go("mandos/undo_last_community_distribution.scen.json");
}

#[test]
fn undo_user_distribution_between_epochs_go() {
    elrond_wasm_debug::mandos_go("mandos/undo_user_distribution_between_epochs.scen.json");
}

#[test]
fn unlock_assets_basic_go() {
    elrond_wasm_debug::mandos_go("mandos/unlock_assets_basic.scen.json");
}
