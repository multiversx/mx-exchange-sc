multiversx_sc::imports!();

use common_errors::{ERROR_BAD_INPUT_TOKEN, ERROR_PARAMETERS};
use core::cmp::Ordering;

use crate::{
    amm, config,
    errors::{ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST, ERROR_SAFE_PRICE_SAME_ROUNDS},
    safe_price::{self, PriceObservation, Round, MAX_OBSERVATIONS},
};

pub const DEFAULT_SAFE_PRICE_ROUNDS_OFFSET: u64 = 10 * 60;
pub const SECONDS_PER_ROUND: u64 = 6;

#[multiversx_sc::module]
pub trait SafePriceViewModule:
    safe_price::SafePriceModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + amm::AmmModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
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
        timestamp_offset: u64,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment> {
        let current_round = self.blockchain().get_block_round();
        let round_offset = timestamp_offset / SECONDS_PER_ROUND;
        require!(
            round_offset > 0 && round_offset < current_round,
            ERROR_PARAMETERS
        );
        let start_round = current_round - round_offset;

        self.get_lp_tokens_safe_price(pair_address, start_round, current_round, liquidity)
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

        let lp_total_supply = self.lp_token_supply().get_from_address(&pair_address);
        let first_token_id = self.first_token_id().get_from_address(&pair_address);
        let second_token_id = self.second_token_id().get_from_address(&pair_address);
        if lp_total_supply == 0 {
            return (
                EsdtTokenPayment::new(first_token_id, 0, BigUint::zero()),
                EsdtTokenPayment::new(second_token_id, 0, BigUint::zero()),
            )
                .into();
        }

        let safe_price_current_index = self
            .safe_price_current_index()
            .get_from_address(&pair_address);
        let price_observations = self.price_observations();

        let last_price_observation = self.get_price_observation(
            &pair_address,
            &first_token_id,
            &second_token_id,
            safe_price_current_index,
            &price_observations,
            end_round,
        );

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
            &first_token_id,
            &second_token_id,
            safe_price_current_index,
            &price_observations,
            start_round,
        );

        let (weighted_first_token_reserve, weighted_second_token_reserve) =
            self.compute_weighted_reserves(&first_price_observation, &last_price_observation);

        let first_token_worth = &liquidity * &weighted_first_token_reserve / &lp_total_supply;
        let second_token_worth = &liquidity * &weighted_second_token_reserve / &lp_total_supply;
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
        round_offset: u64,
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
        timestamp_offset: u64,
        input_payment: EsdtTokenPayment,
    ) -> EsdtTokenPayment {
        let current_round = self.blockchain().get_block_round();
        let round_offset = timestamp_offset / SECONDS_PER_ROUND;
        require!(
            round_offset > 0 && round_offset < current_round,
            ERROR_PARAMETERS
        );
        let start_round = current_round - round_offset;
        self.get_safe_price(pair_address, start_round, current_round, input_payment)
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
            .safe_price_current_index()
            .get_from_address(&pair_address);
        let price_observations = self.price_observations();

        let oldest_price_observation = self.get_oldest_price_observation(
            &pair_address,
            safe_price_current_index,
            &price_observations,
        );
        require!(
            oldest_price_observation.recording_round <= start_round,
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        let first_token_id = self.first_token_id().get_from_address(&pair_address);
        let second_token_id = self.second_token_id().get_from_address(&pair_address);
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
            .safe_price_current_index()
            .get_from_address(&pair_address);
        let first_token_id = self.first_token_id().get_from_address(&pair_address);
        let second_token_id = self.second_token_id().get_from_address(&pair_address);
        let price_observations = self.price_observations();

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
        let first_token_id = self.first_token_id().get_from_address(pair_address);
        let second_token_id = self.second_token_id().get_from_address(pair_address);

        let (weighted_first_token_reserve, weighted_second_token_reserve) =
            self.compute_weighted_reserves(first_price_observation, last_price_observation);

        if input_payment.token_identifier == first_token_id {
            let output_amount =
                input_payment.amount * weighted_second_token_reserve / weighted_first_token_reserve;
            EsdtTokenPayment::new(second_token_id, 0, output_amount)
        } else if input_payment.token_identifier == second_token_id {
            let output_amount =
                input_payment.amount * weighted_first_token_reserve / weighted_second_token_reserve;
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
        price_observations: &VecMapper<Self::Api, PriceObservation<Self::Api>>,
        search_round: Round,
    ) -> PriceObservation<Self::Api> {
        require!(
            !price_observations.is_empty_at_address(pair_address),
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        // Check if the requested price observation is the last one
        let last_observation = price_observations.get_at_address(pair_address, current_index);
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
                .pair_reserve(first_token_id)
                .get_from_address(pair_address);
            let second_token_reserve = self
                .pair_reserve(second_token_id)
                .get_from_address(pair_address);
            return self.compute_new_observation(
                search_round,
                &first_token_reserve,
                &second_token_reserve,
                &last_observation,
            );
        }

        let (mut price_observation, last_search_index) = self.price_observation_by_binary_search(
            pair_address,
            current_index,
            price_observations,
            search_round,
        );

        if price_observation.recording_round > 0 {
            return price_observation;
        }

        price_observation = self.price_observation_by_linear_interpolation(
            pair_address,
            price_observations,
            search_round,
            last_search_index,
        );

        price_observation
    }

    fn get_oldest_price_observation(
        &self,
        pair_address: &ManagedAddress,
        current_index: usize,
        price_observations: &VecMapper<Self::Api, PriceObservation<Self::Api>>,
    ) -> PriceObservation<Self::Api> {
        require!(
            !price_observations.is_empty_at_address(pair_address),
            ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST
        );

        // VecMapper index starts at 1
        let mut oldest_observation_index = 1;
        if price_observations.len_at_address(pair_address) == MAX_OBSERVATIONS {
            oldest_observation_index = (current_index % MAX_OBSERVATIONS) + 1
        }
        price_observations.get_at_address(pair_address, oldest_observation_index)
    }

    fn price_observation_by_binary_search(
        &self,
        pair_address: &ManagedAddress,
        current_index: usize,
        price_observations: &VecMapper<Self::Api, PriceObservation<Self::Api>>,
        search_round: Round,
    ) -> (PriceObservation<Self::Api>, usize) {
        let mut search_index = 1;
        let mut left_index;
        let mut right_index;
        let observation_at_index_1 = price_observations.get_at_address(pair_address, search_index);
        if observation_at_index_1.recording_round <= search_round {
            left_index = search_index;
            right_index = current_index - 1;
        } else {
            left_index = current_index + 1;
            right_index = price_observations.len_at_address(pair_address);
        }

        while left_index <= right_index {
            search_index = (left_index + right_index) / 2;
            let price_observation = price_observations.get_at_address(pair_address, search_index);
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
        price_observations: &VecMapper<Self::Api, PriceObservation<Self::Api>>,
        search_round: Round,
        search_index: usize,
    ) -> PriceObservation<Self::Api> {
        let last_found_observation = price_observations.get_at_address(pair_address, search_index);
        let left_observation;
        let right_observation;
        if last_found_observation.recording_round < search_round {
            left_observation = last_found_observation;
            let right_observation_index = (search_index % MAX_OBSERVATIONS) + 1;
            right_observation =
                price_observations.get_at_address(pair_address, right_observation_index);
        } else {
            let left_observation_index = if search_index == 1 {
                MAX_OBSERVATIONS
            } else {
                search_index - 1
            };
            left_observation =
                price_observations.get_at_address(pair_address, left_observation_index);
            right_observation = last_found_observation;
        };

        // For a proper linear interpolation calculation, we compute the weights as follows
        // Left observation has a weight equal to the remaining time, starting from the searched round until the end round
        // Right observation has a weight equal to the elapsed time, from starting round until the searched round
        let left_weight = right_observation.recording_round - search_round;
        let right_weight = search_round - left_observation.recording_round;

        let weight_sum = left_weight + right_weight;
        let first_token_reserve_sum = BigUint::from(left_weight)
            * left_observation.first_token_reserve_accumulated
            + BigUint::from(right_weight) * right_observation.first_token_reserve_accumulated;
        let second_token_reserve_sum = BigUint::from(left_weight)
            * left_observation.second_token_reserve_accumulated
            + BigUint::from(right_weight) * right_observation.second_token_reserve_accumulated;

        let first_token_reserve_accumulated = first_token_reserve_sum / weight_sum;
        let second_token_reserve_accumulated = second_token_reserve_sum / weight_sum;
        let weight_accumulated =
            left_observation.weight_accumulated + search_round - left_observation.recording_round;

        PriceObservation {
            first_token_reserve_accumulated,
            second_token_reserve_accumulated,
            weight_accumulated,
            recording_round: search_round,
        }
    }

    fn compute_weighted_reserves(
        &self,
        first_price_observation: &PriceObservation<Self::Api>,
        last_price_observation: &PriceObservation<Self::Api>,
    ) -> (BigUint, BigUint) {
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
        (weighted_first_token_reserve, weighted_second_token_reserve)
    }

    fn get_default_offset_rounds(&self, pair_address: &ManagedAddress, end_round: Round) -> u64 {
        let safe_price_current_index = self
            .safe_price_current_index()
            .get_from_address(pair_address);
        let price_observations = self.price_observations();
        let oldest_price_observation = self.get_oldest_price_observation(
            pair_address,
            safe_price_current_index,
            &price_observations,
        );

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
    ) -> MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> {
        let pair_address = self.blockchain().get_sc_address();
        self.get_lp_tokens_safe_price_by_default_offset(pair_address, liquidity)
    }

    #[endpoint(updateAndGetSafePrice)]
    fn update_and_get_safe_price(
        &self,
        input: EsdtTokenPayment<Self::Api>,
    ) -> EsdtTokenPayment<Self::Api> {
        let pair_address = self.blockchain().get_sc_address();
        self.get_safe_price_by_default_offset(pair_address, input)
    }
}
