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

pub const DEFAULT_SAFE_PRICE_ROUNDS_OFFSET: u64 = 10 * 60;
pub const OFFSET_PRECISION_FACTOR: u64 = 1_000_000;

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
        let current_round = self.blockchain().get_block_round();
        let default_offset_rounds = self.get_default_offset_rounds(&pair_address, current_round);
        let start_round = current_round - default_offset_rounds;

        self.get_lp_tokens_safe_price(pair_address, start_round, current_round, liquidity)
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
        let target_observation =
            self.get_observation_by_timestamp_offset(timestamp_offset, pair_address.clone());

        let current_round = self.blockchain().get_block_round();
        self.get_lp_tokens_safe_price(
            pair_address,
            target_observation.recording_round,
            current_round,
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
        require!(end_round > start_round, ERROR_PARAMETERS);

        let first_token_id = self.get_first_token_id_mapper(pair_address.clone()).get();
        let second_token_id = self.get_second_token_id_mapper(pair_address.clone()).get();

        let safe_price_current_index = self
            .get_safe_price_current_index_mapper(pair_address.clone())
            .get();
        let price_observations = self.get_price_observation_mapper(pair_address.clone());

        let oldest_price_observation =
            self.get_oldest_price_observation(safe_price_current_index, &price_observations);

        require!(
            start_round >= oldest_price_observation.recording_round,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let first_price_observation = self.get_price_observation(
            &pair_address,
            &first_token_id,
            &second_token_id,
            safe_price_current_index,
            &price_observations,
            start_round,
        );

        let last_price_observation = self.get_price_observation(
            &pair_address,
            &first_token_id,
            &second_token_id,
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
        let current_round = self.blockchain().get_block_round();
        let default_offset_rounds = self.get_default_offset_rounds(&pair_address, current_round);
        let start_round = current_round - default_offset_rounds;
        self.get_safe_price(pair_address, start_round, current_round, input_payment)
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
        let target_observation =
            self.get_observation_by_timestamp_offset(timestamp_offset, pair_address.clone());

        let current_round = self.blockchain().get_block_round();
        self.get_safe_price(
            pair_address,
            target_observation.recording_round,
            current_round,
            input_payment,
        )
    }

    fn get_observation_by_timestamp_offset(
        &self,
        timestamp_offset: Timestamp,
        pair_address: ManagedAddress,
    ) -> PriceObservation<Self::Api> {
        let current_timestamp = self.blockchain().get_block_timestamp();
        require!(
            timestamp_offset > 0 && timestamp_offset < current_timestamp,
            ERROR_PARAMETERS
        );

        let target_timestamp = current_timestamp - timestamp_offset;

        let safe_price_current_index = self
            .get_safe_price_current_index_mapper(pair_address.clone())
            .get();
        let price_observations = self.get_price_observation_mapper(pair_address);

        self.find_observation_by_timestamp(
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

        let oldest_price_observation =
            self.get_oldest_price_observation(safe_price_current_index, &price_observations);
        require!(
            oldest_price_observation.recording_round <= start_round,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let first_token_id = self.get_first_token_id_mapper(pair_address.clone()).get();
        let second_token_id = self.get_second_token_id_mapper(pair_address.clone()).get();
        let first_price_observation = self.get_price_observation(
            &pair_address,
            &first_token_id,
            &second_token_id,
            safe_price_current_index,
            &price_observations,
            start_round,
        );
        let last_price_observation = self.get_price_observation(
            &pair_address,
            &first_token_id,
            &second_token_id,
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
        let first_token_id = self.get_first_token_id_mapper(pair_address.clone()).get();
        let second_token_id = self.get_second_token_id_mapper(pair_address.clone()).get();
        let price_observations = self.get_price_observation_mapper(pair_address.clone());

        let oldest_price_observation =
            self.get_oldest_price_observation(safe_price_current_index, &price_observations);
        require!(
            oldest_price_observation.recording_round <= search_round,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        self.get_price_observation(
            &pair_address,
            &first_token_id,
            &second_token_id,
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
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
        current_index: usize,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        search_round: Round,
    ) -> PriceObservation<Self::Api> {
        require!(
            !price_observations.is_empty(),
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        // Check if the requested price observation is the last one
        let last_observation = price_observations.get(current_index);
        if last_observation.recording_round == search_round {
            return last_observation;
        }

        // Simulate a new price observation, based on the current reserves,
        // in case the searched round is bigger than the last recording round
        // The search round is limited to the current blockchain round
        if last_observation.recording_round < search_round {
            let current_round = self.blockchain().get_block_round();
            require!(
                search_round <= current_round,
                ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
            );

            let first_token_reserve = self
                .get_pair_reserve_mapper(pair_address.clone(), first_token_id)
                .get();
            let second_token_reserve = self
                .get_pair_reserve_mapper(pair_address.clone(), second_token_id)
                .get();
            let current_lp_supply = self.get_lp_token_supply_mapper(pair_address.clone()).get();
            return self.compute_new_observation(
                search_round,
                &first_token_reserve,
                &second_token_reserve,
                &current_lp_supply,
                &last_observation,
            );
        }

        let (mut price_observation, last_search_index) = self.price_observation_by_binary_search(
            current_index,
            price_observations,
            search_round,
        );

        if price_observation.recording_round > 0 {
            return price_observation;
        }

        price_observation = self.price_observation_by_linear_interpolation(
            price_observations,
            search_round,
            last_search_index,
        );

        price_observation
    }

    fn get_oldest_price_observation(
        &self,
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
        price_observations.get(oldest_observation_index)
    }

    fn price_observation_by_binary_search(
        &self,
        current_index: usize,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        search_round: Round,
    ) -> (PriceObservation<Self::Api>, usize) {
        let mut search_index = 1;
        let mut left_index;
        let mut right_index;
        let observation_at_index_1 = price_observations.get(search_index);
        if observation_at_index_1.recording_round <= search_round {
            left_index = search_index;
            right_index = current_index - 1;
        } else {
            left_index = current_index + 1;
            right_index = price_observations.len();
        }

        while left_index <= right_index {
            search_index = (left_index + right_index) / 2;
            let price_observation = price_observations.get(search_index);
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
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
        search_round: Round,
        search_index: usize,
    ) -> PriceObservation<Self::Api> {
        let last_found_observation = price_observations.get(search_index);
        let left_observation;
        let right_observation;
        if last_found_observation.recording_round < search_round {
            left_observation = last_found_observation;
            let right_observation_index = (search_index % MAX_OBSERVATIONS) + 1;
            right_observation = price_observations.get(right_observation_index);
        } else {
            let left_observation_index = if search_index == 1 {
                MAX_OBSERVATIONS
            } else {
                search_index - 1
            };
            left_observation = price_observations.get(left_observation_index);
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
        let weight_accumulated =
            left_observation.weight_accumulated + search_round - left_observation.recording_round;

        PriceObservation {
            first_token_reserve_accumulated,
            second_token_reserve_accumulated,
            weight_accumulated,
            recording_round: search_round,
            recording_timestamp,
            lp_supply_accumulated,
        }
    }

    fn find_observation_by_timestamp(
        &self,
        target_timestamp: Timestamp,
        current_index: usize,
        price_observations: &VecMapper<PriceObservation<Self::Api>, ManagedAddress>,
    ) -> PriceObservation<Self::Api> {
        require!(
            !price_observations.is_empty(),
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let last_observation = price_observations.get(current_index);
        if last_observation.recording_timestamp <= target_timestamp {
            return last_observation;
        }

        let mut search_index = 1;
        let mut left_index;
        let mut right_index;
        let observation_at_index_1 = price_observations.get(search_index);

        if observation_at_index_1.recording_timestamp <= target_timestamp {
            left_index = search_index;
            right_index = current_index;
        } else {
            left_index = current_index;
            right_index = price_observations.len();
        }

        let mut closest_observation = observation_at_index_1.clone();
        let mut min_timestamp_diff =
            if target_timestamp > observation_at_index_1.recording_timestamp {
                target_timestamp - observation_at_index_1.recording_timestamp
            } else {
                observation_at_index_1.recording_timestamp - target_timestamp
            };

        while left_index <= right_index {
            search_index = (left_index + right_index) / 2;
            let current_observation = price_observations.get(search_index);
            let current_timestamp_diff =
                if target_timestamp > current_observation.recording_timestamp {
                    target_timestamp - current_observation.recording_timestamp
                } else {
                    current_observation.recording_timestamp - target_timestamp
                };

            if current_timestamp_diff < min_timestamp_diff {
                min_timestamp_diff = current_timestamp_diff;
                closest_observation = current_observation.clone();
            }

            match current_observation
                .recording_timestamp
                .cmp(&target_timestamp)
            {
                Ordering::Equal => return current_observation,
                Ordering::Less => left_index = search_index + 1,
                Ordering::Greater => right_index = search_index - 1,
            }
        }

        closest_observation
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

    fn get_default_offset_rounds(&self, pair_address: &ManagedAddress, end_round: Round) -> Round {
        let safe_price_current_index = self
            .get_safe_price_current_index_mapper(pair_address.clone())
            .get();
        let price_observations = self.get_price_observation_mapper(pair_address.clone());
        let oldest_price_observation =
            self.get_oldest_price_observation(safe_price_current_index, &price_observations);

        let mut default_offset_rounds = end_round - oldest_price_observation.recording_round;
        if default_offset_rounds > DEFAULT_SAFE_PRICE_ROUNDS_OFFSET {
            default_offset_rounds = DEFAULT_SAFE_PRICE_ROUNDS_OFFSET;
        }

        default_offset_rounds
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
