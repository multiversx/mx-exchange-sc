use elrond_wasm::*;
use elrond_wasm_debug::*;

fn blockchain_mock() -> BlockchainMock {
    let mut blockchain = BlockchainMock::new();
    blockchain.set_current_dir_from_workspace("locked-asset");

    blockchain.register_contract(
        "file:../dex/farm/output/farm.wasm",
        Box::new(|context| Box::new(farm::contract_obj(context))),
    );
    blockchain.register_contract(
        "file:../dex/pair/output/pair.wasm",
        Box::new(|context| Box::new(pair::contract_obj(context))),
    );
    blockchain.register_contract(
        "file:../dex/router/output/router.wasm",
        Box::new(|context| Box::new(router::contract_obj(context))),
    );
    blockchain.register_contract(
        "file:distribution/output/distribution.wasm",
        Box::new(|context| Box::new(distribution::contract_obj(context))),
    );
    blockchain.register_contract(
        "file:proxy-dex/output/proxy-dex.wasm",
        Box::new(|context| Box::new(proxy_dex::contract_obj(context))),
    );
    blockchain.register_contract(
        "file:factory/output/factory.wasm",
        Box::new(|context| Box::new(factory::contract_obj(context))),
    );
    blockchain
}

#[test]
fn add_liquidity_proxy_rs() {
    elrond_wasm_debug::mandos_rs("mandos/add_liquidity_proxy.scen.json", blockchain_mock());
}

#[test]
fn claim_locked_assets_basic_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/claim_locked_assets_basic.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn claim_mex_rewards_proxy_after_mint_rewards_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/claim_mex_rewards_proxy_after_mint_rewards.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn claim_only_last_four_rs() {
    elrond_wasm_debug::mandos_rs("mandos/claim_only_last_four.scen.json", blockchain_mock());
}

#[test]
fn claim_rewards_proxy_rs() {
    elrond_wasm_debug::mandos_rs("mandos/claim_rewards_proxy.scen.json", blockchain_mock());
}

#[test]
fn claim_rewards_proxy_after_enter_with_lock_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/claim_rewards_proxy_after_enter_with_lock.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn claim_rewards_proxy_after_enter_with_lock_after_mint_rewards_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/claim_rewards_proxy_after_enter_with_lock_after_mint_rewards.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn claim_rewards_proxy_after_mint_rewards_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/claim_rewards_proxy_after_mint_rewards.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn clear_unclaimable_assets_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/clear_unclaimable_assets.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn compound_mex_rewards_proxy_after_mint_rewards_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/compound_mex_rewards_proxy_after_mint_rewards.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn enter_farm_and_lock_rewards_proxy_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/enter_farm_and_lock_rewards_proxy.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn enter_farm_proxy_rs() {
    elrond_wasm_debug::mandos_rs("mandos/enter_farm_proxy.scen.json", blockchain_mock());
}

#[test]
fn enter_mex_farm_proxy_rs() {
    elrond_wasm_debug::mandos_rs("mandos/enter_mex_farm_proxy.scen.json", blockchain_mock());
}

#[test]
fn exit_farm_proxy_rs() {
    elrond_wasm_debug::mandos_rs("mandos/exit_farm_proxy.scen.json", blockchain_mock());
}

#[test]
fn exit_mex_farm_proxy_after_compound_rewards_and_epoch_increase_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/exit_mex_farm_proxy_after_compound_rewards_and_epoch_increase.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn exit_mex_farm_proxy_after_compound_rewards_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/exit_mex_farm_proxy_after_compound_rewards.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn exit_farm_proxy_after_mint_rewards_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/exit_farm_proxy_after_mint_rewards.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn exit_farm_proxy_with_lock_rewards_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/exit_farm_proxy_with_lock_rewards.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn exit_farm_proxy_with_lock_rewards_after_mint_rewards_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/exit_farm_proxy_with_lock_rewards_after_mint_rewards.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn exit_mex_farm_proxy_after_mint_rewards_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/exit_mex_farm_proxy_after_mint_rewards.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn merge_locked_mex_tokens_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/merge_locked_mex_tokens.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn multiple_claim_assets_rs() {
    elrond_wasm_debug::mandos_rs("mandos/multiple_claim_assets.scen.json", blockchain_mock());
}

#[test]
fn remove_liquidity_proxy_rs() {
    elrond_wasm_debug::mandos_rs("mandos/remove_liquidity_proxy.scen.json", blockchain_mock());
}

#[test]
fn remove_liquidity_proxy_after_swap_mex_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/remove_liquidity_proxy_after_swap_mex.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn remove_liquidity_proxy_after_swap_wegld_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/remove_liquidity_proxy_after_swap_wegld.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn set_user_distribution_rs() {
    elrond_wasm_debug::mandos_rs("mandos/set_user_distribution.scen.json", blockchain_mock());
}

#[test]
fn set_user_distribution_above_cap_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/set_user_distribution_above_cap.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn set_user_distribution_duplicate_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/set_user_distribution_duplicate.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn set_user_distribution_with_unlock_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/set_user_distribution_with_unlock.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn undo_last_community_distribution_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/undo_last_community_distribution.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn undo_user_distribution_between_epochs_rs() {
    elrond_wasm_debug::mandos_rs(
        "mandos/undo_user_distribution_between_epochs.scen.json",
        blockchain_mock(),
    );
}

#[test]
fn unlock_assets_basic_rs() {
    elrond_wasm_debug::mandos_rs("mandos/unlock_assets_basic.scen.json", blockchain_mock());
}
