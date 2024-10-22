use common_structs::{Epoch, Timestamp};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EpochToTimestampModule {
    #[endpoint(updateAndGetTimestampStartEpoch)]
    fn update_and_get_timestamp_start_epoch(&self) -> Timestamp {
        let current_epoch = self.blockchain().get_block_epoch();
        let last_update_epoch = self.epoch_last_interaction().get();
        if current_epoch == last_update_epoch {
            return self.timestamp_start_epoch_last_interaction().get();
        }

        self.epoch_last_interaction().set(current_epoch);

        let current_timestamp = self.blockchain().get_block_timestamp();
        self.timestamp_start_epoch_last_interaction()
            .set(current_timestamp);

        current_timestamp
    }

    #[storage_mapper("epochLastInteraction")]
    fn epoch_last_interaction(&self) -> SingleValueMapper<Epoch>;

    #[storage_mapper("timestampStartEpochLastInter")]
    fn timestamp_start_epoch_last_interaction(&self) -> SingleValueMapper<Timestamp>;
}
