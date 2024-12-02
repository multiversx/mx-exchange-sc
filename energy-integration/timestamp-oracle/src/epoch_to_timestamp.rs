use common_structs::{Epoch, Timestamp};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EpochToTimestampModule {
    #[only_owner]
    #[endpoint(setStartTimestampForEpoch)]
    fn set_start_timestamp_for_epoch(&self, epoch: Epoch, start_timestamp: Timestamp) {
        let mapper = self.timestamp_for_epoch(epoch);
        require!(
            mapper.is_empty(),
            "Overwriting timestamp. If you're sure about this, use the overwriteStartTimestampForEpoch endpoint"
        );

        mapper.set(start_timestamp);
    }

    #[only_owner]
    #[endpoint(overwriteStartTimestampForEpoch)]
    fn overwrite_start_timestamp_for_epoch(&self, epoch: Epoch, start_timestamp: Timestamp) {
        self.timestamp_for_epoch(epoch).set(start_timestamp);
    }

    #[endpoint(updateAndGetTimestampStartEpoch)]
    fn update_and_get_timestamp_start_epoch(&self) -> Timestamp {
        let current_epoch = self.blockchain().get_block_epoch();
        let last_update_epoch = self.epoch_last_interaction().get();
        let mapper = self.timestamp_for_epoch(current_epoch);
        if current_epoch == last_update_epoch {
            return mapper.get();
        }

        self.epoch_last_interaction().set(current_epoch);

        let current_timestamp = self.blockchain().get_block_timestamp();
        mapper.set(current_timestamp);

        current_timestamp
    }

    #[view(getStartTimestampForEpoch)]
    fn get_start_timestamp_for_epoch(&self, epoch: Epoch) -> Option<Timestamp> {
        let mapper = self.timestamp_for_epoch(epoch);
        if !mapper.is_empty() {
            Some(mapper.get())
        } else {
            None
        }
    }

    #[view(getStartTimestampMultipleEpochs)]
    fn get_start_timestamp_multiple_epochs(
        &self,
        epochs: MultiValueEncoded<Epoch>,
    ) -> MultiValueEncoded<Option<Timestamp>> {
        let mut timestamps = MultiValueEncoded::new();
        for epoch in epochs {
            let opt_timestamp = self.get_start_timestamp_for_epoch(epoch);
            timestamps.push(opt_timestamp);
        }

        timestamps
    }

    #[storage_mapper("epochLastInteraction")]
    fn epoch_last_interaction(&self) -> SingleValueMapper<Epoch>;

    #[storage_mapper("timestampForEpoch")]
    fn timestamp_for_epoch(&self, epoch: Epoch) -> SingleValueMapper<Timestamp>;
}
