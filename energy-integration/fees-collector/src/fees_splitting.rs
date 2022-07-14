elrond_wasm::imports!();

// use crate::week_timekeeping::Week;

#[elrond_wasm::module]
pub trait FeesSplittingModule:
    crate::config::ConfigModule + crate::week_timekeeping::WeekTimekeepingModule
{
}
