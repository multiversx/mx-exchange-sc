#![no_std]

multiversx_sc::imports!();

use core::convert::TryInto;

pub use common_types::{Epoch, Week};

pub const EPOCHS_IN_WEEK: Epoch = 7;
pub const FIRST_WEEK: Week = 1;
static INVALID_WEEK_ERR_MSG: &[u8] = b"Week 0 is not a valid week";

#[multiversx_sc::module]
pub trait WeekTimekeepingModule {
    /// Week starts from 1
    #[view(getCurrentWeek)]
    fn get_current_week(&self) -> Week {
        let current_epoch = self.blockchain().get_block_epoch();
        self.get_week_for_epoch(current_epoch)
    }

    fn get_week_for_epoch(&self, epoch: Epoch) -> Week {
        let first_week_start_epoch = self.first_week_start_epoch().get();
        require!(epoch >= first_week_start_epoch, INVALID_WEEK_ERR_MSG);

        unsafe {
            // will never overflow usize
            let zero_based_week: Week = ((epoch - first_week_start_epoch) / EPOCHS_IN_WEEK)
                .try_into()
                .unwrap_unchecked();

            zero_based_week + 1
        }
    }

    fn get_start_epoch_for_week(&self, week: Week) -> Epoch {
        require!(week != 0, INVALID_WEEK_ERR_MSG);

        let first_week_start_epoch = self.first_week_start_epoch().get();
        first_week_start_epoch + (week - 1) as u64 * EPOCHS_IN_WEEK
    }

    fn get_end_epoch_for_week(&self, week: Week) -> Epoch {
        let start_epoch = self.get_start_epoch_for_week(week);
        start_epoch + EPOCHS_IN_WEEK - 1
    }

    #[view(getFirstWeekStartEpoch)]
    #[storage_mapper("firstWeekStartEpoch")]
    fn first_week_start_epoch(&self) -> SingleValueMapper<Epoch>;
}
