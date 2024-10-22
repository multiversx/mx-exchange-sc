#![no_std]

use common_structs::Timestamp;

multiversx_sc::imports!();

pub mod epoch_to_timestamp;

#[multiversx_sc::contract]
pub trait TimestampOracle: epoch_to_timestamp::EpochToTimestampModule {
    #[init]
    fn init(&self, current_epoch_start_timestamp: Timestamp) {
        let current_epoch = self.blockchain().get_block_epoch();
        self.epoch_last_interaction().set(current_epoch);
        self.timestamp_start_epoch_last_interaction()
            .set(current_epoch_start_timestamp);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
