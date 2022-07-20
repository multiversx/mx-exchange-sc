#![no_std]
#![feature(generic_associated_types)]

elrond_wasm::imports!();

pub mod config;
pub mod fees_accumulation;
pub mod fees_splitting;
pub mod week_timekeeping;

#[elrond_wasm::contract]
pub trait FeesCollector:
    config::ConfigModule
    + week_timekeeping::WeekTimekeepingModule
    + fees_splitting::FeesSplittingModule
    + fees_accumulation::FeesAccumulationModule
    + energy_query_module::EnergyQueryModule
{
    #[init]
    fn init(&self) {
        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set_if_empty(current_epoch);
    }
}
