multiversx_sc::imports!();

use common_errors::{ERROR_BAD_INPUT_TOKEN, ERROR_PARAMETERS};
use core::cmp::Ordering;
use math::weighted_average;

use crate::{
    amm, config,
    errors::{
        ERROR_SAFE_PRICE_CURRENT_INDEX, ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST,
        ERROR_SAFE_PRICE_TIMESTAMP_ORDER, ERROR_SAFE_PRICE_WEIGHT_ORDER,
    },
    read_pair_storage,
    safe_price::{
        self, PriceObservation, Round, Timestamp, LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS,
        MAX_OBSERVATIONS,
    },
};

const MILLISECONDS_PER_SECOND: u64 = 1_000;

struct PriceObservationWeightedAmounts<M: ManagedTypeApi> {
    weighted_first_token_reserve: BigUint<M>,
    weighted_second_token_reserve: BigUint<M>,
    weighted_lp_supply: BigUint<M>,
}

struct RoundTimestampContext {
    anchor_round: Round,
    anchor_timestamp: Timestamp,
    current_round: Round,
    current_round_duration: u64,
    legacy_rounds: u64,
}

struct PriceObservationReadContext<M: ManagedTypeApi> {
    current_index: usize,
    oldest_observation: PriceObservation<M>,
    last_recorded_observation: PriceObservation<M>,
    latest_observation: PriceObservation<M>,
    legacy_cutover: (Round, Timestamp),
}

#[multiversx_sc::module]
pub trait SafePriceViewModule:
    safe_price::SafePriceModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + amm::AmmModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
    + read_pair_storage::ReadPairStorageModule
{
    #[label("safe-price-view")]
    #[view(getLpTokensSafePriceByDefaultOffset)]
    fn get_lp_tokens_safe_price_by_default_offset(
        &self,
        pair_address: ManagedAddress,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment> {
        let default_timestamp_offset = self.get_default_timestamp_offset(&pair_address);
        self.get_lp_tokens_safe_price_by_timestamp_offset_ms(
            pair_address,
            default_timestamp_offset,
            liquidity,
        )
    }

    #[label("safe-price-view")]
    #[view(getLpTokensSafePriceByRoundOffset)]
    fn get_lp_tokens_safe_price_by_round_offset(
        &self,
        pair_address: ManagedAddress,
        round_offset: Round,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment> {
        let (start_round, current_round) =
            self.get_range_by_offset(self.blockchain().get_block_round(), round_offset);
        self.get_lp_tokens_safe_price(pair_address, start_round, current_round, liquidity)
    }

    #[label("safe-price-view")]
    #[view(getLpTokensSafePriceByTimestampOffset)]
    fn get_lp_tokens_safe_price_by_timestamp_offset(
        &self,
        pair_address: ManagedAddress,
        timestamp_offset_seconds: Timestamp,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment> {
        self.get_lp_tokens_safe_price_by_timestamp_offset_ms(
            pair_address,
            self.seconds_to_milliseconds(timestamp_offset_seconds),
            liquidity,
        )
    }

    #[label("safe-price-view")]
    #[view(getLpTokensSafePriceByTimestampOffsetMs)]
    fn get_lp_tokens_safe_price_by_timestamp_offset_ms(
        &self,
        pair_address: ManagedAddress,
        timestamp_offset_milliseconds: Timestamp,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment> {
        let (start_timestamp, end_timestamp) = self.get_range_by_offset(
            self.get_current_timestamp_milliseconds(),
            timestamp_offset_milliseconds,
        );
        self.get_lp_tokens_safe_price_by_timestamp_range(
            pair_address,
            start_timestamp,
            end_timestamp,
            liquidity,
        )
    }

    #[label("safe-price-view")]
    #[view(getLpTokensSafePrice)]
    fn get_lp_tokens_safe_price(
        &self,
        pair_address: ManagedAddress,
        start_round: Round,
        end_round: Round,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment> {
        let (first_price_observation, last_price_observation) =
            self.load_price_observations_by_round_range(&pair_address, start_round, end_round);

        self.compute_lp_tokens_safe_price(
            pair_address,
            liquidity,
            &first_price_observation,
            &last_price_observation,
        )
    }

    fn get_lp_tokens_safe_price_by_timestamp_range(
        &self,
        pair_address: ManagedAddress,
        start_timestamp: Timestamp,
        end_timestamp: Timestamp,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment> {
        let (first_price_observation, last_price_observation) = self
            .load_price_observations_by_timestamp_range(
                &pair_address,
                start_timestamp,
                end_timestamp,
            );

        self.compute_lp_tokens_safe_price(
            pair_address,
            liquidity,
            &first_price_observation,
            &last_price_observation,
        )
    }

    fn compute_lp_tokens_safe_price(
        &self,
        pair_address: ManagedAddress,
        liquidity: BigUint,
        first_price_observation: &PriceObservation<Self::Api>,
        last_price_observation: &PriceObservation<Self::Api>,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment> {
        let first_token_id = self.get_first_token_id_mapper(pair_address.clone()).get();
        let second_token_id = self.get_second_token_id_mapper(pair_address.clone()).get();

        let mut weighted_amounts =
            self.compute_weighted_amounts(first_price_observation, last_price_observation);

        if weighted_amounts.weighted_lp_supply == 0 {
            weighted_amounts.weighted_lp_supply =
                self.get_lp_token_supply_mapper(pair_address.clone()).get();
        }
        if weighted_amounts.weighted_lp_supply == 0 {
            return (
                EsdtTokenPayment::new(first_token_id, 0, BigUint::zero()),
                EsdtTokenPayment::new(second_token_id, 0, BigUint::zero()),
            )
                .into();
        }

        let first_token_worth = &liquidity * &weighted_amounts.weighted_first_token_reserve
            / &weighted_amounts.weighted_lp_supply;
        let second_token_worth = &liquidity * &weighted_amounts.weighted_second_token_reserve
            / &weighted_amounts.weighted_lp_supply;
        let first_token_payment = EsdtTokenPayment::new(first_token_id, 0, first_token_worth);
        let second_token_payment = EsdtTokenPayment::new(second_token_id, 0, second_token_worth);

        (first_token_payment, second_token_payment).into()
    }

    #[label("safe-price-view")]
    #[view(getSafePriceByDefaultOffset)]
    fn get_safe_price_by_default_offset(
        &self,
        pair_address: ManagedAddress,
        input_payment: EsdtTokenPayment,
    ) -> EsdtTokenPayment {
        let default_timestamp_offset = self.get_default_timestamp_offset(&pair_address);
        self.get_safe_price_by_timestamp_offset_ms(
            pair_address,
            default_timestamp_offset,
            input_payment,
        )
    }

    #[label("safe-price-view")]
    #[view(getSafePriceByRoundOffset)]
    fn get_safe_price_by_round_offset(
        &self,
        pair_address: ManagedAddress,
        round_offset: Round,
        input_payment: EsdtTokenPayment,
    ) -> EsdtTokenPayment {
        let (start_round, current_round) =
            self.get_range_by_offset(self.blockchain().get_block_round(), round_offset);
        self.get_safe_price(pair_address, start_round, current_round, input_payment)
    }

    #[label("safe-price-view")]
    #[view(getSafePriceByTimestampOffset)]
    fn get_safe_price_by_timestamp_offset(
        &self,
        pair_address: ManagedAddress,
        timestamp_offset_seconds: Timestamp,
        input_payment: EsdtTokenPayment,
    ) -> EsdtTokenPayment {
        self.get_safe_price_by_timestamp_offset_ms(
            pair_address,
            self.seconds_to_milliseconds(timestamp_offset_seconds),
            input_payment,
        )
    }

    #[label("safe-price-view")]
    #[view(getSafePriceByTimestampOffsetMs)]
    fn get_safe_price_by_timestamp_offset_ms(
        &self,
        pair_address: ManagedAddress,
        timestamp_offset_milliseconds: Timestamp,
        input_payment: EsdtTokenPayment,
    ) -> EsdtTokenPayment {
        let (start_timestamp, end_timestamp) = self.get_range_by_offset(
            self.get_current_timestamp_milliseconds(),
            timestamp_offset_milliseconds,
        );
        self.get_safe_price_by_timestamp_range(
            pair_address,
            start_timestamp,
            end_timestamp,
            input_payment,
        )
    }

    #[label("safe-price-view")]
    #[view(getSafePrice)]
    fn get_safe_price(
        &self,
        pair_address: ManagedAddress,
        start_round: Round,
        end_round: Round,
        input_payment: EsdtTokenPayment,
    ) -> EsdtTokenPayment {
        let (first_price_observation, last_price_observation) =
            self.load_price_observations_by_round_range(&pair_address, start_round, end_round);

        self.compute_weighted_price(
            &pair_address,
            input_payment,
            &first_price_observation,
            &last_price_observation,
        )
    }

    #[label("safe-price-view")]
    #[view(getPriceObservation)]
    fn get_price_observation_view(
        &self,
        pair_address: ManagedAddress,
        search_round: Round,
    ) -> PriceObservation<Self::Api> {
        let (price_observations, read_context) =
            self.load_price_observation_search_context(&pair_address);
        let timestamp_context = self.get_round_timestamp_context(&read_context.oldest_observation);

        self.get_price_observation_by_round(
            &pair_address,
            &price_observations,
            search_round,
            &timestamp_context,
            &read_context,
        )
    }

    fn get_range_by_offset(&self, current: u64, offset: u64) -> (u64, u64) {
        require!(offset > 0 && offset < current, ERROR_PARAMETERS);
        (current - offset, current)
    }

    fn seconds_to_milliseconds(&self, seconds: Timestamp) -> Timestamp {
        let Some(milliseconds) = seconds.checked_mul(MILLISECONDS_PER_SECOND) else {
            sc_panic!(ERROR_PARAMETERS);
        };
        milliseconds
    }

    fn get_safe_price_by_timestamp_range(
        &self,
        pair_address: ManagedAddress,
        start_timestamp: Timestamp,
        end_timestamp: Timestamp,
        input_payment: EsdtTokenPayment,
    ) -> EsdtTokenPayment {
        let (first_price_observation, last_price_observation) = self
            .load_price_observations_by_timestamp_range(
                &pair_address,
                start_timestamp,
                end_timestamp,
            );

        self.compute_weighted_price(
            &pair_address,
            input_payment,
            &first_price_observation,
            &last_price_observation,
        )
    }

    fn compute_weighted_price(
        &self,
        pair_address: &ManagedAddress,
        input_payment: EsdtTokenPayment,
        first_price_observation: &PriceObservation<Self::Api>,
        last_price_observation: &PriceObservation<Self::Api>,
    ) -> EsdtTokenPayment {
        let first_token_id = self.get_first_token_id_mapper(pair_address.clone()).get();
        let second_token_id = self.get_second_token_id_mapper(pair_address.clone()).get();

        let weighted_amounts =
            self.compute_weighted_amounts(first_price_observation, last_price_observation);

        let (output_token_id, input_reserve, output_reserve) =
            if input_payment.token_identifier == first_token_id {
                (
                    second_token_id,
                    weighted_amounts.weighted_first_token_reserve,
                    weighted_amounts.weighted_second_token_reserve,
                )
            } else if input_payment.token_identifier == second_token_id {
                (
                    first_token_id,
                    weighted_amounts.weighted_second_token_reserve,
                    weighted_amounts.weighted_first_token_reserve,
                )
            } else {
                sc_panic!(ERROR_BAD_INPUT_TOKEN);
            };
        let output_amount = input_payment.amount * output_reserve / input_reserve;

        EsdtTokenPayment::new(output_token_id, 0, output_amount)
    }

    fn load_price_observations_by_timestamp_range(
        &self,
        pair_address: &ManagedAddress,
        start_timestamp: Timestamp,
        end_timestamp: Timestamp,
    ) -> (PriceObservation<Self::Api>, PriceObservation<Self::Api>) {
        require!(end_timestamp > start_timestamp, ERROR_PARAMETERS);

        let (price_observations, read_context) =
            self.load_price_observation_search_context(pair_address);

        (
            self.get_price_observation_by_timestamp(
                pair_address,
                &price_observations,
                start_timestamp,
                &read_context,
            ),
            self.get_price_observation_by_timestamp(
                pair_address,
                &price_observations,
                end_timestamp,
                &read_context,
            ),
        )
    }

    fn load_price_observations_by_round_range(
        &self,
        pair_address: &ManagedAddress,
        start_round: Round,
        end_round: Round,
    ) -> (PriceObservation<Self::Api>, PriceObservation<Self::Api>) {
        require!(end_round > start_round, ERROR_PARAMETERS);

        let (price_observations, read_context) =
            self.load_price_observation_search_context(pair_address);
        let timestamp_context = self.get_round_timestamp_context(&read_context.oldest_observation);

        (
            self.get_price_observation_by_round(
                pair_address,
                &price_observations,
                start_round,
                &timestamp_context,
                &read_context,
            ),
            self.get_price_observation_by_round(
                pair_address,
                &price_observations,
                end_round,
                &timestamp_context,
                &read_context,
            ),
        )
    }

    fn get_price_observation_by_round(
        &self,
        pair_address: &ManagedAddress,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        target_round: Round,
        timestamp_context: &RoundTimestampContext,
        read_context: &PriceObservationReadContext<Self::Api>,
    ) -> PriceObservation<Self::Api> {
        let target_timestamp = self.infer_timestamp_for_round(timestamp_context, target_round);
        let mut observation = self.get_price_observation_by_timestamp(
            pair_address,
            price_observations,
            target_timestamp,
            read_context,
        );
        observation.recording_round = target_round;
        observation
    }

    fn get_round_timestamp_context(
        &self,
        oldest_observation: &PriceObservation<Self::Api>,
    ) -> RoundTimestampContext {
        let current_round = self.blockchain().get_block_round();
        let current_timestamp = self.get_current_timestamp_milliseconds();
        let current_round_duration = self.get_current_round_duration_milliseconds();
        require!(
            oldest_observation.recording_timestamp > 0
                && oldest_observation.recording_round <= current_round
                && oldest_observation.recording_timestamp <= current_timestamp,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let elapsed_rounds = current_round - oldest_observation.recording_round;
        let elapsed_milliseconds = current_timestamp - oldest_observation.recording_timestamp;
        let legacy_round_duration = LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS;

        // E = legacy_rounds * L + (D - legacy_rounds) * S.
        // Solving it yields the exact cadence-transition round without another stored anchor.
        let legacy_rounds = if current_round_duration == legacy_round_duration {
            require!(
                elapsed_milliseconds.is_multiple_of(legacy_round_duration)
                    && elapsed_milliseconds / legacy_round_duration == elapsed_rounds,
                ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
            );
            elapsed_rounds
        } else {
            require!(
                current_round_duration > 0 && current_round_duration < legacy_round_duration,
                ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
            );
            require!(
                elapsed_rounds <= elapsed_milliseconds / current_round_duration,
                ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
            );
            let current_duration_elapsed = elapsed_rounds * current_round_duration;
            let legacy_duration_elapsed = elapsed_milliseconds - current_duration_elapsed;
            let duration_difference = legacy_round_duration - current_round_duration;
            require!(
                legacy_duration_elapsed.is_multiple_of(duration_difference),
                ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
            );
            let legacy_rounds = legacy_duration_elapsed / duration_difference;
            require!(
                legacy_rounds <= elapsed_rounds,
                ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
            );
            legacy_rounds
        };

        RoundTimestampContext {
            anchor_round: oldest_observation.recording_round,
            anchor_timestamp: oldest_observation.recording_timestamp,
            current_round,
            current_round_duration,
            legacy_rounds,
        }
    }

    fn infer_timestamp_for_round(
        &self,
        timestamp_context: &RoundTimestampContext,
        target_round: Round,
    ) -> Timestamp {
        require!(
            timestamp_context.anchor_round <= target_round
                && target_round <= timestamp_context.current_round,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let round_offset = target_round - timestamp_context.anchor_round;
        let legacy_rounds = core::cmp::min(round_offset, timestamp_context.legacy_rounds);
        let current_rounds = round_offset - legacy_rounds;

        timestamp_context.anchor_timestamp
            + legacy_rounds * LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS
            + current_rounds * timestamp_context.current_round_duration
    }

    fn get_price_observation_by_timestamp(
        &self,
        pair_address: &ManagedAddress,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        target_timestamp: Timestamp,
        read_context: &PriceObservationReadContext<Self::Api>,
    ) -> PriceObservation<Self::Api> {
        require!(
            read_context.oldest_observation.recording_timestamp > 0
                && target_timestamp >= read_context.oldest_observation.recording_timestamp,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let last_observation = &read_context.last_recorded_observation;
        let latest_observation = &read_context.latest_observation;

        if last_observation.recording_timestamp == target_timestamp {
            return (*last_observation).clone();
        }

        if latest_observation.recording_timestamp == target_timestamp {
            return (*latest_observation).clone();
        }

        if latest_observation.recording_timestamp < target_timestamp {
            let current_timestamp = self.get_current_timestamp_milliseconds();
            require!(
                target_timestamp <= current_timestamp
                    && current_timestamp > latest_observation.recording_timestamp,
                ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
            );

            let first_token_id = self.get_first_token_id_mapper(pair_address.clone()).get();
            let second_token_id = self.get_second_token_id_mapper(pair_address.clone()).get();

            let first_token_reserve = self
                .get_pair_reserve_mapper(pair_address.clone(), &first_token_id)
                .get();
            let second_token_reserve = self
                .get_pair_reserve_mapper(pair_address.clone(), &second_token_id)
                .get();
            let current_lp_supply = self.get_lp_token_supply_mapper(pair_address.clone()).get();
            let mut observation = latest_observation.clone();
            self.accumulate_into_observation(
                &mut observation,
                latest_observation.recording_round,
                target_timestamp,
                &first_token_reserve,
                &second_token_reserve,
                &current_lp_supply,
            );
            return observation;
        }

        if last_observation.recording_timestamp < target_timestamp {
            return self.interpolate_price_observation_by_timestamp(
                last_observation,
                latest_observation,
                target_timestamp,
            );
        }

        let (price_observation, last_search_index) = self
            .price_observation_by_timestamp_binary_search(
                price_observations,
                target_timestamp,
                read_context,
            );

        if price_observation.recording_timestamp > 0 {
            return price_observation;
        }

        self.price_observation_by_timestamp_interpolation(
            price_observations,
            target_timestamp,
            last_search_index,
            read_context,
        )
    }

    fn get_price_observation_at_index(
        &self,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        index: usize,
        read_context: &PriceObservationReadContext<Self::Api>,
    ) -> PriceObservation<Self::Api> {
        if index == read_context.current_index {
            return read_context.last_recorded_observation.clone();
        }

        self.normalize_observation(
            price_observations.get(index),
            OptionalValue::Some(read_context.legacy_cutover),
        )
    }

    fn load_price_observation_search_context(
        &self,
        pair_address: &ManagedAddress,
    ) -> (
        VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        PriceObservationReadContext<Self::Api>,
    ) {
        let current_index = self
            .get_safe_price_current_index_mapper(pair_address.clone())
            .get();
        let price_observations = self.get_price_observation_mapper(pair_address.clone());
        let observation_count = price_observations.len();
        require!(
            observation_count > 0,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );
        require!(
            current_index > 0
                && current_index <= observation_count
                && current_index <= MAX_OBSERVATIONS,
            ERROR_SAFE_PRICE_CURRENT_INDEX
        );

        let cutover_mapper = self.get_safe_price_legacy_cutover_mapper(pair_address.clone());
        let legacy_cutover = if cutover_mapper.is_empty() {
            (0, 0)
        } else {
            cutover_mapper.get()
        };
        let last_recorded_observation = self.normalize_observation(
            price_observations.get(current_index),
            OptionalValue::Some(legacy_cutover),
        );
        let current_price_observation_mapper =
            self.get_current_price_observation_mapper(pair_address.clone());
        require!(
            !current_price_observation_mapper.is_empty(),
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );
        let latest_observation = current_price_observation_mapper.get();
        require!(
            latest_observation.recording_timestamp >= last_recorded_observation.recording_timestamp,
            ERROR_SAFE_PRICE_TIMESTAMP_ORDER
        );
        let oldest_observation_index = if observation_count == MAX_OBSERVATIONS {
            (current_index % MAX_OBSERVATIONS) + 1
        } else {
            1
        };
        let oldest_observation = if oldest_observation_index == current_index {
            last_recorded_observation.clone()
        } else {
            self.normalize_observation(
                price_observations.get(oldest_observation_index),
                OptionalValue::Some(legacy_cutover),
            )
        };
        let read_context = PriceObservationReadContext {
            current_index,
            oldest_observation,
            last_recorded_observation,
            latest_observation,
            legacy_cutover,
        };

        (price_observations, read_context)
    }

    fn price_observation_by_timestamp_binary_search(
        &self,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        target_timestamp: Timestamp,
        read_context: &PriceObservationReadContext<Self::Api>,
    ) -> (PriceObservation<Self::Api>, usize) {
        let observation_at_index_1 =
            self.get_price_observation_at_index(price_observations, 1, read_context);
        require!(
            observation_at_index_1.recording_timestamp > 0,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let (mut left_index, mut right_index) =
            if observation_at_index_1.recording_timestamp <= target_timestamp {
                (1, read_context.current_index - 1)
            } else {
                (read_context.current_index + 1, price_observations.len())
            };
        let mut search_index = 1;

        while left_index <= right_index {
            search_index = (left_index + right_index) / 2;
            let price_observation =
                self.get_price_observation_at_index(price_observations, search_index, read_context);
            match price_observation.recording_timestamp.cmp(&target_timestamp) {
                Ordering::Equal => return (price_observation, search_index),
                Ordering::Less => left_index = search_index + 1,
                Ordering::Greater => right_index = search_index - 1,
            }
        }

        (PriceObservation::default(), search_index)
    }

    fn price_observation_by_timestamp_interpolation(
        &self,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        target_timestamp: Timestamp,
        search_index: usize,
        read_context: &PriceObservationReadContext<Self::Api>,
    ) -> PriceObservation<Self::Api> {
        let last_found_observation =
            self.get_price_observation_at_index(price_observations, search_index, read_context);

        let (left_observation, right_observation) =
            if last_found_observation.recording_timestamp < target_timestamp {
                let right_observation_index = (search_index % MAX_OBSERVATIONS) + 1;
                (
                    last_found_observation,
                    self.get_price_observation_at_index(
                        price_observations,
                        right_observation_index,
                        read_context,
                    ),
                )
            } else {
                let left_observation_index = if search_index == 1 {
                    MAX_OBSERVATIONS
                } else {
                    search_index - 1
                };
                (
                    self.get_price_observation_at_index(
                        price_observations,
                        left_observation_index,
                        read_context,
                    ),
                    last_found_observation,
                )
            };

        self.interpolate_price_observation_by_timestamp(
            &left_observation,
            &right_observation,
            target_timestamp,
        )
    }

    fn interpolate_price_observation_by_timestamp(
        &self,
        left_observation: &PriceObservation<Self::Api>,
        right_observation: &PriceObservation<Self::Api>,
        target_timestamp: Timestamp,
    ) -> PriceObservation<Self::Api> {
        require!(
            left_observation.recording_timestamp > 0
                && right_observation.recording_timestamp > left_observation.recording_timestamp
                && target_timestamp > left_observation.recording_timestamp
                && target_timestamp < right_observation.recording_timestamp,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let left_weight = right_observation.recording_timestamp - target_timestamp;
        let right_weight = target_timestamp - left_observation.recording_timestamp;
        require!(
            right_observation.weight_accumulated >= left_observation.weight_accumulated
                && right_observation.weight_accumulated - left_observation.weight_accumulated
                    == right_observation.recording_timestamp - left_observation.recording_timestamp,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );
        let left_weight_biguint = BigUint::from(left_weight);
        let right_weight_biguint = BigUint::from(right_weight);

        PriceObservation {
            first_token_reserve_accumulated: weighted_average(
                left_observation.first_token_reserve_accumulated.clone(),
                left_weight_biguint.clone(),
                right_observation.first_token_reserve_accumulated.clone(),
                right_weight_biguint.clone(),
            ),
            second_token_reserve_accumulated: weighted_average(
                left_observation.second_token_reserve_accumulated.clone(),
                left_weight_biguint.clone(),
                right_observation.second_token_reserve_accumulated.clone(),
                right_weight_biguint.clone(),
            ),
            weight_accumulated: right_observation.weight_accumulated - left_weight,
            recording_round: left_observation.recording_round,
            recording_timestamp: target_timestamp,
            lp_supply_accumulated: weighted_average(
                left_observation.lp_supply_accumulated.clone(),
                left_weight_biguint,
                right_observation.lp_supply_accumulated.clone(),
                right_weight_biguint,
            ),
        }
    }

    fn compute_weighted_amounts(
        &self,
        first_price_observation: &PriceObservation<Self::Api>,
        last_price_observation: &PriceObservation<Self::Api>,
    ) -> PriceObservationWeightedAmounts<Self::Api> {
        require!(
            last_price_observation.weight_accumulated > first_price_observation.weight_accumulated,
            ERROR_SAFE_PRICE_WEIGHT_ORDER
        );
        let weight_diff =
            last_price_observation.weight_accumulated - first_price_observation.weight_accumulated;

        let first_token_reserve_diff = &last_price_observation.first_token_reserve_accumulated
            - &first_price_observation.first_token_reserve_accumulated;
        let second_token_reserve_diff = &last_price_observation.second_token_reserve_accumulated
            - &first_price_observation.second_token_reserve_accumulated;

        let weighted_first_token_reserve = first_token_reserve_diff / weight_diff;
        let weighted_second_token_reserve = second_token_reserve_diff / weight_diff;

        let weighted_lp_supply = if first_price_observation.lp_supply_accumulated > 0 {
            let lp_supply_diff = &last_price_observation.lp_supply_accumulated
                - &first_price_observation.lp_supply_accumulated;
            lp_supply_diff / weight_diff
        } else {
            BigUint::zero()
        };

        PriceObservationWeightedAmounts {
            weighted_first_token_reserve,
            weighted_second_token_reserve,
            weighted_lp_supply,
        }
    }

    fn get_default_timestamp_offset(&self, pair_address: &ManagedAddress) -> Timestamp {
        let router_address = self.get_pair_router_mapper(pair_address.clone()).get();
        let default_safe_price_timestamp_offset = self
            .get_default_safe_price_timestamp_offset_mapper(router_address)
            .get();

        require!(
            default_safe_price_timestamp_offset > 0,
            "Default safe price timestamp offset not set"
        );

        let current_timestamp = self.get_current_timestamp_milliseconds();
        let (_, read_context) = self.load_price_observation_search_context(pair_address);
        require!(
            read_context.oldest_observation.recording_timestamp > 0,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let available_offset =
            current_timestamp.saturating_sub(read_context.oldest_observation.recording_timestamp);

        if available_offset > 0 && available_offset < default_safe_price_timestamp_offset {
            return available_offset;
        }

        default_safe_price_timestamp_offset
    }

    // legacy endpoints

    #[endpoint(updateAndGetTokensForGivenPositionWithSafePrice)]
    fn update_and_get_tokens_for_given_position_with_safe_price(
        &self,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment> {
        let pair_address = self.blockchain().get_sc_address();
        self.get_lp_tokens_safe_price_by_default_offset(pair_address, liquidity)
    }

    #[endpoint(updateAndGetSafePrice)]
    fn update_and_get_safe_price(&self, input: EsdtTokenPayment) -> EsdtTokenPayment {
        let pair_address = self.blockchain().get_sc_address();
        self.get_safe_price_by_default_offset(pair_address, input)
    }
}
