multiversx_sc::imports!();

use common_errors::{ERROR_BAD_INPUT_TOKEN, ERROR_PARAMETERS};
use core::cmp::Ordering;
use math::weighted_average;

use crate::{
    amm, config,
    errors::{ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST, ERROR_SAFE_PRICE_SAME_ROUNDS},
    read_pair_storage,
    safe_price::{self, PriceObservation, Round, Timestamp, MAX_OBSERVATIONS},
};

const LEGACY_ROUND_DURATION_MILLISECONDS: u64 =
    safe_price::DEFAULT_SAFE_PRICE_TIMESTAMP_SAVE_INTERVAL_MILLISECONDS;

struct PriceObservationWeightedAmounts<M: ManagedTypeApi> {
    weighted_first_token_reserve: BigUint<M>,
    weighted_second_token_reserve: BigUint<M>,
    weighted_lp_supply: BigUint<M>,
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
        self.get_lp_tokens_safe_price_by_timestamp_offset(
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
        let current_round = self.blockchain().get_block_round();
        require!(
            round_offset > 0 && round_offset < current_round,
            ERROR_PARAMETERS
        );
        let start_round = current_round - round_offset;

        self.get_lp_tokens_safe_price(pair_address, start_round, current_round, liquidity)
    }

    #[label("safe-price-view")]
    #[view(getLpTokensSafePriceByTimestampOffset)]
    fn get_lp_tokens_safe_price_by_timestamp_offset(
        &self,
        pair_address: ManagedAddress,
        timestamp_offset: Timestamp,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment> {
        let target_round =
            self.get_round_by_timestamp_offset(timestamp_offset, pair_address.clone());

        let current_round = self.blockchain().get_block_round();
        self.get_lp_tokens_safe_price(pair_address, target_round, current_round, liquidity)
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
        require!(end_round > start_round, ERROR_PARAMETERS);

        let first_token_id = self.get_first_token_id_mapper(pair_address.clone()).get();
        let second_token_id = self.get_second_token_id_mapper(pair_address.clone()).get();

        let safe_price_current_index = self
            .get_safe_price_current_index_mapper(pair_address.clone())
            .get();
        let price_observations = self.get_price_observation_mapper(pair_address.clone());

        let oldest_price_observation = self.get_oldest_price_observation(
            &pair_address,
            safe_price_current_index,
            &price_observations,
        );

        require!(
            start_round >= oldest_price_observation.recording_round,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let first_price_observation = self.get_price_observation(
            &pair_address,
            safe_price_current_index,
            &price_observations,
            start_round,
        );

        let last_price_observation = self.get_price_observation(
            &pair_address,
            safe_price_current_index,
            &price_observations,
            end_round,
        );

        let mut weighted_amounts =
            self.compute_weighted_amounts(&first_price_observation, &last_price_observation);

        if weighted_amounts.weighted_lp_supply == 0 {
            let current_lp_supply = self.get_lp_token_supply_mapper(pair_address.clone()).get();
            if current_lp_supply == 0 {
                return (
                    EsdtTokenPayment::new(first_token_id, 0, BigUint::zero()),
                    EsdtTokenPayment::new(second_token_id, 0, BigUint::zero()),
                )
                    .into();
            } else {
                weighted_amounts.weighted_lp_supply = current_lp_supply;
            }
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
        self.get_safe_price_by_timestamp_offset(
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
        let current_round = self.blockchain().get_block_round();
        require!(
            round_offset > 0 && round_offset < current_round,
            ERROR_PARAMETERS
        );
        let start_round = current_round - round_offset;
        self.get_safe_price(pair_address, start_round, current_round, input_payment)
    }

    #[label("safe-price-view")]
    #[view(getSafePriceByTimestampOffset)]
    fn get_safe_price_by_timestamp_offset(
        &self,
        pair_address: ManagedAddress,
        timestamp_offset: Timestamp,
        input_payment: EsdtTokenPayment,
    ) -> EsdtTokenPayment {
        let target_round =
            self.get_round_by_timestamp_offset(timestamp_offset, pair_address.clone());

        let current_round = self.blockchain().get_block_round();
        self.get_safe_price(pair_address, target_round, current_round, input_payment)
    }

    fn get_round_by_timestamp_offset(
        &self,
        timestamp_offset: Timestamp,
        pair_address: ManagedAddress,
    ) -> Round {
        let current_timestamp = self.get_current_timestamp_milliseconds();
        require!(
            timestamp_offset > 0 && timestamp_offset < current_timestamp,
            ERROR_PARAMETERS
        );

        let target_timestamp = current_timestamp - timestamp_offset;

        let safe_price_current_index = self
            .get_safe_price_current_index_mapper(pair_address.clone())
            .get();
        let price_observations = self.get_price_observation_mapper(pair_address.clone());
        self.find_equivalent_round_for_timestamp(
            &pair_address,
            target_timestamp,
            safe_price_current_index,
            &price_observations,
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
        require!(end_round > start_round, ERROR_PARAMETERS);

        let safe_price_current_index = self
            .get_safe_price_current_index_mapper(pair_address.clone())
            .get();
        let price_observations = self.get_price_observation_mapper(pair_address.clone());

        let oldest_price_observation = self.get_oldest_price_observation(
            &pair_address,
            safe_price_current_index,
            &price_observations,
        );
        require!(
            oldest_price_observation.recording_round <= start_round,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let first_price_observation = self.get_price_observation(
            &pair_address,
            safe_price_current_index,
            &price_observations,
            start_round,
        );
        let last_price_observation = self.get_price_observation(
            &pair_address,
            safe_price_current_index,
            &price_observations,
            end_round,
        );

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
        let safe_price_current_index = self
            .get_safe_price_current_index_mapper(pair_address.clone())
            .get();
        let price_observations = self.get_price_observation_mapper(pair_address.clone());

        let oldest_price_observation = self.get_oldest_price_observation(
            &pair_address,
            safe_price_current_index,
            &price_observations,
        );
        require!(
            oldest_price_observation.recording_round <= search_round,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        self.get_price_observation(
            &pair_address,
            safe_price_current_index,
            &price_observations,
            search_round,
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

        if input_payment.token_identifier == first_token_id {
            let output_amount = input_payment.amount
                * weighted_amounts.weighted_second_token_reserve
                / weighted_amounts.weighted_first_token_reserve;
            EsdtTokenPayment::new(second_token_id, 0, output_amount)
        } else if input_payment.token_identifier == second_token_id {
            let output_amount = input_payment.amount
                * weighted_amounts.weighted_first_token_reserve
                / weighted_amounts.weighted_second_token_reserve;
            EsdtTokenPayment::new(first_token_id, 0, output_amount)
        } else {
            sc_panic!(ERROR_BAD_INPUT_TOKEN);
        }
    }

    fn get_price_observation(
        &self,
        pair_address: &ManagedAddress,
        current_index: usize,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        search_round: Round,
    ) -> PriceObservation<Self::Api> {
        require!(
            !price_observations.is_empty(),
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let last_observation = self.get_price_observation_from_storage(
            pair_address,
            current_index,
            price_observations,
            current_index,
        );
        let latest_observation = self.get_latest_available_price_observation(
            pair_address,
            &last_observation,
            current_index,
            price_observations,
        );
        if latest_observation.recording_round == search_round {
            return latest_observation;
        }

        // Simulate a new price observation, based on the current reserves,
        // in case the searched round is bigger than the last recording round
        // The search round is limited to the current blockchain round
        if latest_observation.recording_round < search_round {
            let current_round = self.blockchain().get_block_round();
            require!(
                search_round <= current_round,
                ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
            );
            let current_timestamp = self.get_current_timestamp_milliseconds();
            if current_timestamp <= latest_observation.recording_timestamp {
                let mut current_observation = latest_observation;
                current_observation.recording_round = search_round;
                return current_observation;
            }

            let search_timestamp = weighted_average(
                latest_observation.recording_timestamp,
                current_round - search_round,
                current_timestamp,
                search_round - latest_observation.recording_round,
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
            return self.compute_new_observation(
                search_round,
                search_timestamp,
                &first_token_reserve,
                &second_token_reserve,
                &current_lp_supply,
                &latest_observation,
            );
        }

        if last_observation.recording_round < search_round {
            let left_weight = latest_observation.recording_round - search_round;
            let right_weight = search_round - last_observation.recording_round;

            return PriceObservation {
                first_token_reserve_accumulated: weighted_average(
                    last_observation.first_token_reserve_accumulated,
                    BigUint::from(left_weight),
                    latest_observation.first_token_reserve_accumulated,
                    BigUint::from(right_weight),
                ),
                second_token_reserve_accumulated: weighted_average(
                    last_observation.second_token_reserve_accumulated,
                    BigUint::from(left_weight),
                    latest_observation.second_token_reserve_accumulated,
                    BigUint::from(right_weight),
                ),
                weight_accumulated: weighted_average(
                    last_observation.weight_accumulated,
                    left_weight,
                    latest_observation.weight_accumulated,
                    right_weight,
                ),
                recording_round: search_round,
                recording_timestamp: weighted_average(
                    last_observation.recording_timestamp,
                    left_weight,
                    latest_observation.recording_timestamp,
                    right_weight,
                ),
                lp_supply_accumulated: weighted_average(
                    last_observation.lp_supply_accumulated,
                    BigUint::from(left_weight),
                    latest_observation.lp_supply_accumulated,
                    BigUint::from(right_weight),
                ),
            };
        }

        let (price_observation, last_search_index) = self.price_observation_by_binary_search(
            pair_address,
            current_index,
            price_observations,
            search_round,
        );

        if price_observation.recording_round > 0 {
            return price_observation;
        }

        self.price_observation_by_linear_interpolation(
            pair_address,
            current_index,
            price_observations,
            search_round,
            last_search_index,
        )
    }

    fn get_oldest_price_observation(
        &self,
        pair_address: &ManagedAddress,
        current_index: usize,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
    ) -> PriceObservation<Self::Api> {
        require!(
            !price_observations.is_empty(),
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        // VecMapper index starts at 1
        let mut oldest_observation_index = 1;
        if price_observations.len() == MAX_OBSERVATIONS {
            oldest_observation_index = (current_index % MAX_OBSERVATIONS) + 1
        }
        self.get_price_observation_from_storage(
            pair_address,
            current_index,
            price_observations,
            oldest_observation_index,
        )
    }

    fn get_price_observation_from_storage(
        &self,
        pair_address: &ManagedAddress,
        current_index: usize,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        index: usize,
    ) -> PriceObservation<Self::Api> {
        let observation = price_observations.get(index);
        self.infer_legacy_price_observation(
            observation,
            pair_address,
            current_index,
            price_observations,
        )
    }

    fn infer_legacy_price_observation(
        &self,
        mut observation: PriceObservation<Self::Api>,
        pair_address: &ManagedAddress,
        current_index: usize,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
    ) -> PriceObservation<Self::Api> {
        if observation.recording_round == 0 || observation.recording_timestamp > 0 {
            return observation;
        }

        let timestamp_reference = self.get_timestamp_reference_observation(
            pair_address,
            current_index,
            price_observations,
        );
        let recording_timestamp = if timestamp_reference.recording_timestamp > 0
            && timestamp_reference.recording_round >= observation.recording_round
        {
            let elapsed_rounds = timestamp_reference.recording_round - observation.recording_round;
            timestamp_reference
                .recording_timestamp
                .saturating_sub(self.legacy_rounds_to_milliseconds(elapsed_rounds))
        } else {
            let current_round = self.blockchain().get_block_round();
            let current_timestamp = self.get_current_timestamp_milliseconds();
            if current_timestamp == 0 {
                return observation;
            }

            if current_round <= observation.recording_round {
                current_timestamp
            } else {
                let elapsed_rounds = current_round - observation.recording_round;
                current_timestamp.saturating_sub(self.legacy_rounds_to_milliseconds(elapsed_rounds))
            }
        };

        if recording_timestamp == 0 {
            return observation;
        }

        observation.recording_timestamp = recording_timestamp;
        let multiplier = BigUint::from(LEGACY_ROUND_DURATION_MILLISECONDS);
        observation.first_token_reserve_accumulated *= &multiplier;
        observation.second_token_reserve_accumulated *= &multiplier;
        observation.lp_supply_accumulated *= &multiplier;
        observation.weight_accumulated =
            self.legacy_rounds_to_milliseconds(observation.weight_accumulated);
        observation
    }

    fn legacy_rounds_to_milliseconds(&self, rounds: u64) -> u64 {
        match rounds.checked_mul(LEGACY_ROUND_DURATION_MILLISECONDS) {
            Some(value) => value,
            None => sc_panic!("Safe price duration overflow"),
        }
    }

    fn get_timestamp_reference_observation(
        &self,
        pair_address: &ManagedAddress,
        current_index: usize,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
    ) -> PriceObservation<Self::Api> {
        let current_price_observation_mapper =
            self.get_current_price_observation_mapper(pair_address.clone());
        if !current_price_observation_mapper.is_empty() {
            let current_price_observation = current_price_observation_mapper.get();
            if current_price_observation.recording_timestamp > 0 {
                return current_price_observation;
            }
        }

        let last_observation = price_observations.get(current_index);
        if last_observation.recording_timestamp > 0 {
            return last_observation;
        }

        PriceObservation::default()
    }

    fn get_latest_available_price_observation(
        &self,
        pair_address: &ManagedAddress,
        last_recorded_observation: &PriceObservation<Self::Api>,
        current_index: usize,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
    ) -> PriceObservation<Self::Api> {
        let last_recorded_observation = last_recorded_observation.clone();
        let current_price_observation_mapper =
            self.get_current_price_observation_mapper(pair_address.clone());
        if current_price_observation_mapper.is_empty() {
            return last_recorded_observation;
        }

        let current_price_observation = self.infer_legacy_price_observation(
            current_price_observation_mapper.get(),
            pair_address,
            current_index,
            price_observations,
        );
        if current_price_observation.recording_round > last_recorded_observation.recording_round {
            current_price_observation
        } else {
            last_recorded_observation
        }
    }

    fn price_observation_by_binary_search(
        &self,
        pair_address: &ManagedAddress,
        current_index: usize,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        search_round: Round,
    ) -> (PriceObservation<Self::Api>, usize) {
        let mut search_index = 1;
        let mut left_index;
        let mut right_index;
        let observation_at_index_1 = self.get_price_observation_from_storage(
            pair_address,
            current_index,
            price_observations,
            search_index,
        );
        if observation_at_index_1.recording_round <= search_round {
            left_index = search_index;
            right_index = current_index - 1;
        } else {
            left_index = current_index + 1;
            right_index = price_observations.len();
        }

        while left_index <= right_index {
            search_index = (left_index + right_index) / 2;
            let price_observation = self.get_price_observation_from_storage(
                pair_address,
                current_index,
                price_observations,
                search_index,
            );
            match price_observation.recording_round.cmp(&search_round) {
                Ordering::Equal => return (price_observation, search_index),
                Ordering::Less => left_index = search_index + 1,
                Ordering::Greater => right_index = search_index - 1,
            }
        }

        (PriceObservation::default(), search_index)
    }

    fn price_observation_by_linear_interpolation(
        &self,
        pair_address: &ManagedAddress,
        current_index: usize,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        search_round: Round,
        search_index: usize,
    ) -> PriceObservation<Self::Api> {
        let last_found_observation = self.get_price_observation_from_storage(
            pair_address,
            current_index,
            price_observations,
            search_index,
        );
        let left_observation;
        let right_observation;
        if last_found_observation.recording_round < search_round {
            left_observation = last_found_observation;
            let right_observation_index = (search_index % MAX_OBSERVATIONS) + 1;
            right_observation = self.get_price_observation_from_storage(
                pair_address,
                current_index,
                price_observations,
                right_observation_index,
            );
        } else {
            let left_observation_index = if search_index == 1 {
                MAX_OBSERVATIONS
            } else {
                search_index - 1
            };
            left_observation = self.get_price_observation_from_storage(
                pair_address,
                current_index,
                price_observations,
                left_observation_index,
            );
            right_observation = last_found_observation;
        };

        // For a proper linear interpolation calculation, we compute the weights as follows
        // Left observation has a weight equal to the remaining time, starting from the searched round until the end round
        // Right observation has a weight equal to the elapsed time, from starting round until the searched round
        let left_weight = right_observation.recording_round - search_round;
        let right_weight = search_round - left_observation.recording_round;

        let first_token_reserve_accumulated = weighted_average(
            left_observation.first_token_reserve_accumulated,
            BigUint::from(left_weight),
            right_observation.first_token_reserve_accumulated,
            BigUint::from(right_weight),
        );
        let second_token_reserve_accumulated = weighted_average(
            left_observation.second_token_reserve_accumulated,
            BigUint::from(left_weight),
            right_observation.second_token_reserve_accumulated,
            BigUint::from(right_weight),
        );
        let lp_supply_accumulated = weighted_average(
            left_observation.lp_supply_accumulated,
            BigUint::from(left_weight),
            right_observation.lp_supply_accumulated,
            BigUint::from(right_weight),
        );
        let recording_timestamp = weighted_average(
            left_observation.recording_timestamp,
            left_weight,
            right_observation.recording_timestamp,
            right_weight,
        );
        let weight_accumulated = weighted_average(
            left_observation.weight_accumulated,
            left_weight,
            right_observation.weight_accumulated,
            right_weight,
        );

        PriceObservation {
            first_token_reserve_accumulated,
            second_token_reserve_accumulated,
            weight_accumulated,
            recording_round: search_round,
            recording_timestamp,
            lp_supply_accumulated,
        }
    }

    fn find_equivalent_round_for_timestamp(
        &self,
        pair_address: &ManagedAddress,
        target_timestamp: Timestamp,
        current_index: usize,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
    ) -> Round {
        require!(
            !price_observations.is_empty(),
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let oldest_observation =
            self.get_oldest_price_observation(pair_address, current_index, price_observations);
        require!(
            target_timestamp >= oldest_observation.recording_timestamp,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let last_observation = self.get_price_observation_from_storage(
            pair_address,
            current_index,
            price_observations,
            current_index,
        );
        let latest_observation = self.get_latest_available_price_observation(
            pair_address,
            &last_observation,
            current_index,
            price_observations,
        );
        if latest_observation.recording_timestamp == target_timestamp {
            return latest_observation.recording_round;
        }

        if latest_observation.recording_timestamp < target_timestamp {
            let current_timestamp = self.get_current_timestamp_milliseconds();
            require!(
                target_timestamp <= current_timestamp,
                ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
            );

            let current_round = self.blockchain().get_block_round();
            return self.interpolate_round_between_timestamps(
                latest_observation.recording_round,
                latest_observation.recording_timestamp,
                current_round,
                current_timestamp,
                target_timestamp,
            );
        }

        if last_observation.recording_timestamp < target_timestamp {
            return self.interpolate_round_between_timestamps(
                last_observation.recording_round,
                last_observation.recording_timestamp,
                latest_observation.recording_round,
                latest_observation.recording_timestamp,
                target_timestamp,
            );
        }

        let mut search_index = 1;
        let mut left_index;
        let mut right_index;
        let observation_at_index_1 = self.get_price_observation_from_storage(
            pair_address,
            current_index,
            price_observations,
            search_index,
        );
        if observation_at_index_1.recording_timestamp <= target_timestamp {
            left_index = search_index;
            right_index = current_index - 1;
        } else {
            left_index = current_index + 1;
            right_index = price_observations.len();
        }

        while left_index <= right_index {
            search_index = (left_index + right_index) / 2;
            let price_observation = self.get_price_observation_from_storage(
                pair_address,
                current_index,
                price_observations,
                search_index,
            );
            match price_observation.recording_timestamp.cmp(&target_timestamp) {
                Ordering::Equal => return price_observation.recording_round,
                Ordering::Less => left_index = search_index + 1,
                Ordering::Greater => right_index = search_index - 1,
            }
        }

        let last_found_observation = self.get_price_observation_from_storage(
            pair_address,
            current_index,
            price_observations,
            search_index,
        );
        let (left_observation, right_observation) =
            if last_found_observation.recording_timestamp < target_timestamp {
                let right_observation_index = (search_index % MAX_OBSERVATIONS) + 1;
                (
                    last_found_observation,
                    self.get_price_observation_from_storage(
                        pair_address,
                        current_index,
                        price_observations,
                        right_observation_index,
                    ),
                )
            } else {
                let left_observation_index = if search_index == 1 {
                    MAX_OBSERVATIONS
                } else {
                    search_index - 1
                };
                (
                    self.get_price_observation_from_storage(
                        pair_address,
                        current_index,
                        price_observations,
                        left_observation_index,
                    ),
                    last_found_observation,
                )
            };

        self.interpolate_round_between_timestamps(
            left_observation.recording_round,
            left_observation.recording_timestamp,
            right_observation.recording_round,
            right_observation.recording_timestamp,
            target_timestamp,
        )
    }

    fn interpolate_round_between_timestamps(
        &self,
        left_round: Round,
        left_timestamp: Timestamp,
        right_round: Round,
        right_timestamp: Timestamp,
        target_timestamp: Timestamp,
    ) -> Round {
        if right_timestamp <= left_timestamp {
            return left_round;
        }

        let left_weight = right_timestamp - target_timestamp;
        let right_weight = target_timestamp - left_timestamp;

        weighted_average(left_round, left_weight, right_round, right_weight)
    }

    fn compute_weighted_amounts(
        &self,
        first_price_observation: &PriceObservation<Self::Api>,
        last_price_observation: &PriceObservation<Self::Api>,
    ) -> PriceObservationWeightedAmounts<Self::Api> {
        let weight_diff =
            last_price_observation.weight_accumulated - first_price_observation.weight_accumulated;

        require!(weight_diff > 0, ERROR_SAFE_PRICE_SAME_ROUNDS);

        let first_token_reserve_diff = last_price_observation
            .first_token_reserve_accumulated
            .clone()
            - first_price_observation
                .first_token_reserve_accumulated
                .clone();
        let second_token_reserve_diff = last_price_observation
            .second_token_reserve_accumulated
            .clone()
            - first_price_observation
                .second_token_reserve_accumulated
                .clone();

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
        let safe_price_current_index = self
            .get_safe_price_current_index_mapper(pair_address.clone())
            .get();
        let price_observations = self.get_price_observation_mapper(pair_address.clone());
        let oldest_observation = self.get_oldest_price_observation(
            pair_address,
            safe_price_current_index,
            &price_observations,
        );

        let available_offset =
            current_timestamp.saturating_sub(oldest_observation.recording_timestamp);

        if available_offset == 0 {
            default_safe_price_timestamp_offset
        } else {
            core::cmp::min(default_safe_price_timestamp_offset, available_offset)
        }
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
