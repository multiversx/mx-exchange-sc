elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::config;
use crate::contexts::base::StorageCache;
use crate::contexts::swap::SwapContext;

const PERCENT_MAX: u64 = 100_000;

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct BPConfig {
    protect_stop_block: u64,
    volume_percent: u64,
    max_num_actions_per_address: u64,
}

#[elrond_wasm::module]
pub trait BPModule:
    config::ConfigModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
{
    fn require_can_proceed_swap(
        &self,
        ctx: &SwapContext<Self::Api>,
        storage_cache: &StorageCache<Self>,
    ) {
        if self.bp_swap_config().is_empty() {
            return;
        }

        let caller = self.blockchain().get_caller();
        let bp_config = self.bp_swap_config().get();
        let current_block = self.blockchain().get_block_nonce();
        if bp_config.protect_stop_block < current_block {
            self.num_swaps_by_address(&caller).clear();
            return;
        }

        let reserve_in = storage_cache.get_reserve_in(ctx.swap_tokens_order);
        let reserve_out = storage_cache.get_reserve_out(ctx.swap_tokens_order);
        if *reserve_in == 0 && *reserve_out == 0 {
            return;
        }

        let num_swaps = self.num_swaps_by_address(&caller).get();
        require!(
            num_swaps < bp_config.max_num_actions_per_address,
            "too many swaps by address"
        );

        let amount_in_percent = &ctx.final_input_amount * PERCENT_MAX / reserve_in;
        require!(
            amount_in_percent < bp_config.volume_percent,
            "swap amount in too large"
        );

        let amount_out_percent = &ctx.final_output_amount * PERCENT_MAX / reserve_out;
        require!(
            amount_out_percent < bp_config.volume_percent,
            "swap amount out too large"
        );

        self.num_swaps_by_address(&caller).set(num_swaps + 1);
    }

    fn require_can_proceed_remove(&self, lp_token_supply: &BigUint, liquidity_removed: &BigUint) {
        if self.bp_remove_config().is_empty() {
            return;
        }

        let caller = self.blockchain().get_caller();
        let bp_config = self.bp_remove_config().get();
        let current_block = self.blockchain().get_block_nonce();
        if bp_config.protect_stop_block < current_block {
            self.num_removes_by_address(&caller).clear();
            return;
        }
        if lp_token_supply == &0u64 {
            return;
        }

        let num_removes = self.num_removes_by_address(&caller).get();
        require!(
            num_removes < bp_config.max_num_actions_per_address,
            "too many removes by address"
        );

        let percent_removed = liquidity_removed * PERCENT_MAX / lp_token_supply;
        require!(
            percent_removed < bp_config.volume_percent,
            "remove liquidity too large"
        );

        self.num_removes_by_address(&caller).set(num_removes + 1);
    }

    fn require_can_proceed_add(&self, lp_token_supply: &BigUint, liquidity_added: &BigUint) {
        if self.bp_add_config().is_empty() {
            return;
        }

        let caller = self.blockchain().get_caller();
        let bp_config = self.bp_add_config().get();
        let current_block = self.blockchain().get_block_nonce();
        if bp_config.protect_stop_block < current_block {
            self.num_adds_by_address(&caller).clear();
            return;
        }
        if lp_token_supply == &0 {
            return;
        }

        let num_adds = self.num_adds_by_address(&caller).get();
        require!(
            num_adds < bp_config.max_num_actions_per_address,
            "too many adds by address"
        );

        let percent_added = liquidity_added * PERCENT_MAX / lp_token_supply;
        require!(
            percent_added < bp_config.volume_percent,
            "add liquidity too large"
        );

        self.num_adds_by_address(&caller).set(num_adds + 1);
    }

    #[endpoint(setBPSwapConfig)]
    fn set_bp_swap_config(
        &self,
        protect_stop_block: u64,
        volume_percent: u64,
        max_num_actions_per_address: u64,
    ) {
        self.require_caller_has_owner_permissions();
        self.bp_swap_config().set(&BPConfig {
            protect_stop_block,
            volume_percent,
            max_num_actions_per_address,
        });
    }

    #[endpoint(setBPRemoveConfig)]
    fn set_bp_remove_config(
        &self,
        protect_stop_block: u64,
        volume_percent: u64,
        max_num_actions_per_address: u64,
    ) {
        self.require_caller_has_owner_permissions();
        self.bp_remove_config().set(&BPConfig {
            protect_stop_block,
            volume_percent,
            max_num_actions_per_address,
        });
    }

    #[endpoint(setBPAddConfig)]
    fn set_bp_add_config(
        &self,
        protect_stop_block: u64,
        volume_percent: u64,
        max_num_actions_per_address: u64,
    ) {
        self.require_caller_has_owner_permissions();
        self.bp_add_config().set(&BPConfig {
            protect_stop_block,
            volume_percent,
            max_num_actions_per_address,
        });
    }

    #[view(getBPSwapConfig)]
    #[storage_mapper("bp_swap_config")]
    fn bp_swap_config(&self) -> SingleValueMapper<BPConfig>;

    #[view(getNumSwapsByAddress)]
    #[storage_mapper("num_swaps_by_address")]
    fn num_swaps_by_address(&self, address: &ManagedAddress) -> SingleValueMapper<u64>;

    #[view(getBPRemoveConfig)]
    #[storage_mapper("bp_remove_config")]
    fn bp_remove_config(&self) -> SingleValueMapper<BPConfig>;

    #[view(getNumRemovesByAddress)]
    #[storage_mapper("num_removes_by_address")]
    fn num_removes_by_address(&self, address: &ManagedAddress) -> SingleValueMapper<u64>;

    #[view(getBPAddConfig)]
    #[storage_mapper("bp_add_config")]
    fn bp_add_config(&self) -> SingleValueMapper<BPConfig>;

    #[view(getNumAddsByAddress)]
    #[storage_mapper("num_adds_by_address")]
    fn num_adds_by_address(&self, address: &ManagedAddress) -> SingleValueMapper<u64>;
}
