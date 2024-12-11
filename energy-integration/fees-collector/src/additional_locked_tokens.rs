multiversx_sc::imports!();

use common_types::Week;

pub const BLOCKS_IN_WEEK: u64 = 100_800;

#[multiversx_sc::module]
pub trait AdditionalLockedTokensModule:
    crate::config::ConfigModule
    + crate::fees_accumulation::FeesAccumulationModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
    + crate::external_sc_interactions::router::RouterInteractionsModule
    + crate::external_sc_interactions::pair::PairInteractionsModule
    + utils::UtilsModule
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

        let blocks_in_week = BLOCKS_IN_WEEK;
        let amount_per_block = self.locked_tokens_per_block().get();
        let new_tokens_amount = amount_per_block * blocks_in_week;

        let locked_token_id = self.locked_token_id().get();
        self.accumulated_fees(current_week - 1, &locked_token_id)
            .update(|fees| *fees += new_tokens_amount);

        last_update_week_mapper.set(current_week);
    }

    #[view(getLastLockedTokensAddWeek)]
    #[storage_mapper("lastLockedTokenAddWeek")]
    fn last_locked_token_add_week(&self) -> SingleValueMapper<Week>;

    #[view(getLockedTokensPerBlock)]
    #[storage_mapper("lockedTokensPerBlock")]
    fn locked_tokens_per_block(&self) -> SingleValueMapper<BigUint>;
}
