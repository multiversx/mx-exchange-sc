use common_types::{Nonce, Week};

elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait AdditionalLockedTokensModule:
    crate::config::ConfigModule
    + crate::fees_accumulation::FeesAccumulationModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
{
    #[only_owner]
    #[endpoint(setLockedTokensPerBlock)]
    fn set_locked_tokens_per_block(&self, locked_tokens_per_block: BigUint) {
        self.accumulate_additional_locked_tokens();
        self.locked_tokens_per_block().set(locked_tokens_per_block);
    }

    fn accumulate_additional_locked_tokens(&self) {
        let last_update_week_mapper = self.last_locked_token_add_week();
        let last_update_week = last_update_week_mapper.get();
        let current_week = self.get_current_week();
        if last_update_week == current_week {
            return;
        }

        let last_update_block_mapper = self.last_locked_tokens_add_block();
        let last_block = last_update_block_mapper.get();
        let current_block = self.blockchain().get_block_nonce();

        let block_diff = current_block - last_block;
        let amount_per_block = self.locked_tokens_per_block().get();
        let new_tokens_amount = amount_per_block * block_diff;

        let locked_token_id = self.locked_token_id().get();
        self.accumulated_fees(current_week, &locked_token_id)
            .update(|fees| *fees += new_tokens_amount);

        last_update_week_mapper.set(current_week);
        last_update_block_mapper.set(current_block);
    }

    #[view(getLastLockedTokensAddBlock)]
    #[storage_mapper("lastLockedTokenAddBlock")]
    fn last_locked_tokens_add_block(&self) -> SingleValueMapper<Nonce>;

    #[view(getLastLockedTokensAddWeek)]
    #[storage_mapper("lastLockedTokenAddWeek")]
    fn last_locked_token_add_week(&self) -> SingleValueMapper<Week>;

    #[view(getLockedTokensPerBlock)]
    #[storage_mapper("lockedTokensPerBlock")]
    fn locked_tokens_per_block(&self) -> SingleValueMapper<BigUint>;
}
