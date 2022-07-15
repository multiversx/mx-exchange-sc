elrond_wasm::imports!();

use core::convert::TryInto;

pub type Week = usize;
pub type Epoch = u64;

pub const EPOCHS_IN_WEEK: Epoch = 7;

#[elrond_wasm::module]
pub trait WeekTimekeepingModule {
    /// Week starts from 1
    #[view(getCurrentWeek)]
    fn get_current_week(&self) -> Week {
        let first_week_start_epoch = self.first_week_start_epoch().get();
        let current_epoch = self.blockchain().get_block_epoch();

        unsafe {
            // will never overflow usize
            let zero_based_week: Week = ((current_epoch - first_week_start_epoch) / EPOCHS_IN_WEEK)
                .try_into()
                .unwrap_unchecked();

            zero_based_week + 1
        }
    }

    #[storage_mapper("firstWeekStartEpoch")]
    fn first_week_start_epoch(&self) -> SingleValueMapper<Epoch>;
}
