elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::amm;
use super::config;
use super::liquidity_pool;

#[elrond_wasm::module]
pub trait SafeReserveModule:
    config::ConfigModule
    + liquidity_pool::LiquidityPoolModule
    + token_send::TokenSendModule
    + token_supply::TokenSupplyModule
    + amm::AmmModule
{
    fn update_safe_reserve(&self) {
        let last_block = self.last_block().get();
        let current_block = self.blockchain().get_block_nonce();

        if last_block == current_block {
            return;
        }

        let first_reserve = self.pair_reserve(&self.first_token_id().get()).get();
        let second_reserve = self.pair_reserve(&self.second_token_id().get()).get();

        let num_blocks = self.num_blocks().get();
        if num_blocks == 0 {
            self.num_blocks().set(&1u64);
            self.safe_reserves_first().set(&first_reserve);
            self.safe_reserves_second().set(&second_reserve);
            return;
        }

        let blocks_passed = current_block - last_block;
        let safe_reserve_first = self.safe_reserves_first().get();
        let safe_reserve_second = self.safe_reserves_second().get();

        let new_safe_reserve_first = (safe_reserve_first * num_blocks.into()
            + first_reserve * blocks_passed.into())
            / (num_blocks + blocks_passed).into();

        let new_safe_reserve_second = (safe_reserve_second * num_blocks.into()
            + second_reserve * blocks_passed.into())
            / (num_blocks + blocks_passed).into();

        self.last_block().set(&last_block);
        self.num_blocks().set(&(num_blocks + blocks_passed));
        self.safe_reserves_first().set(&new_safe_reserve_first);
        self.safe_reserves_second().set(&new_safe_reserve_second);
    }

    fn reset_safe_reserve(&self) {
        self.num_blocks().clear();
        self.safe_reserves_first().clear();
        self.safe_reserves_second().clear();
    }

    #[storage_mapper("SafeReserveModule:last_block")]
    fn last_block(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("SafeReserveModule:num_blocks")]
    fn num_blocks(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("SafeReserveModule:reserves_first")]
    fn safe_reserves_first(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[storage_mapper("SafeReserveModule:reserves_second")]
    fn safe_reserves_second(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;
}
