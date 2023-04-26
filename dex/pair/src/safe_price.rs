multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    amm, config,
    errors::{
        ERROR_SAFE_PRICE_MAX_OBSERVATIONS, ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST,
        ERROR_UNKNOWN_TOKEN, ERROR_ZERO_AMOUNT,
    },
};

pub const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000_000_000;

type Round = u64;

#[derive(Clone, TopEncode, TopDecode, TypeAbi)]
pub struct SafePriceInfo {
    pub current_index: usize,
    pub max_observations: usize,
    pub default_safe_price_rounds_offset: u64,
    pub division_safety_constant: u64,
}

impl Default for SafePriceInfo {
    fn default() -> Self {
        SafePriceInfo {
            current_index: 0,
            max_observations: 0,
            default_safe_price_rounds_offset: 0,
            division_safety_constant: 0,
        }
    }
}

#[derive(ManagedVecItem, Clone, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi)]
pub struct PriceObservation<M: ManagedTypeApi> {
    pub first_token_price_accumulated: BigUint<M>,
    pub second_token_price_accumulated: BigUint<M>,
    pub weight_accumulated: u64,
    pub recording_round: Round,
}

impl<M: ManagedTypeApi> Default for PriceObservation<M> {
    fn default() -> Self {
        PriceObservation {
            first_token_price_accumulated: BigUint::zero(),
            second_token_price_accumulated: BigUint::zero(),
            weight_accumulated: 0,
            recording_round: 0,
        }
    }
}

impl<M: ManagedTypeApi> PriceObservation<M> {
    fn compute_new_observation(
        &mut self,
        new_round: Round,
        new_first_reserve: &BigUint<M>,
        new_second_reserve: &BigUint<M>,
        division_safety_constant: u64,
    ) {
        let new_weight = new_round - self.recording_round;
        self.first_token_price_accumulated += BigUint::from(new_weight)
            * (new_second_reserve * division_safety_constant / new_first_reserve);
        self.second_token_price_accumulated += BigUint::from(new_weight)
            * (new_first_reserve * division_safety_constant / new_second_reserve);
        self.weight_accumulated += new_weight;
        self.recording_round = new_round;
    }
}

#[multiversx_sc::module]
pub trait SafePriceModule:
    config::ConfigModule
    + token_send::TokenSendModule
    + amm::AmmModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
{
    #[endpoint(updateAndGetTokensForGivenPositionWithSafePrice)]
    fn update_and_get_tokens_for_given_position_with_safe_price(
        &self,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> {
        let lp_total_supply = self.lp_token_supply().get();
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        if lp_total_supply != 0 {
            return MultiValue2::from((
                EsdtTokenPayment::new(first_token_id, 0, BigUint::zero()),
                EsdtTokenPayment::new(second_token_id, 0, BigUint::zero()),
            ));
        }

        self.update_safe_state_on_the_fly();

        let safe_price_info = self.safe_price_info().get();
        let current_round = self.blockchain().get_block_round();
        let offset_round = current_round - safe_price_info.default_safe_price_rounds_offset;
        let price_observations = self.price_observations().get();

        let first_price_observation = self.get_price_observation(
            safe_price_info.current_index,
            safe_price_info.max_observations,
            &price_observations,
            offset_round,
        );
        let last_price_observation = self.get_price_observation(
            safe_price_info.current_index,
            safe_price_info.max_observations,
            &price_observations,
            current_round,
        );

        let first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_reserve = self.pair_reserve(&second_token_id).get();
        let first_token_worth = &liquidity * &first_token_reserve / &lp_total_supply;
        let second_token_worth = &liquidity * &second_token_reserve / &lp_total_supply;

        let first_token_payment = EsdtTokenPayment::new(first_token_id, 0, first_token_worth);
        let second_token_payment = EsdtTokenPayment::new(second_token_id, 0, second_token_worth);
        let first_token_weighted = self.compute_weighted_price(
            &first_price_observation,
            &last_price_observation,
            safe_price_info.division_safety_constant,
            first_token_payment,
        );

        let second_token_weighted = self.compute_weighted_price(
            &first_price_observation,
            &last_price_observation,
            safe_price_info.division_safety_constant,
            second_token_payment,
        );

        MultiValue2::from((first_token_weighted, second_token_weighted))
    }

    #[endpoint(updateAndGetSafePrice)]
    fn update_and_get_safe_price(
        &self,
        start_round: Round,
        end_round: Round,
        input: EsdtTokenPayment<Self::Api>,
    ) -> EsdtTokenPayment<Self::Api> {
        self.update_safe_state_on_the_fly();

        let safe_price_info = self.safe_price_info().get();
        let price_observations = self.price_observations().get();
        let first_price_observation = self.get_price_observation(
            safe_price_info.current_index,
            safe_price_info.max_observations,
            &price_observations,
            start_round,
        );
        let last_price_observation = self.get_price_observation(
            safe_price_info.current_index,
            safe_price_info.max_observations,
            &price_observations,
            end_round,
        );

        self.compute_weighted_price(
            &first_price_observation,
            &last_price_observation,
            safe_price_info.division_safety_constant,
            input,
        )
    }

    fn compute_weighted_price(
        &self,
        first_price_observation: &PriceObservation<Self::Api>,
        last_price_observation: &PriceObservation<Self::Api>,
        division_safety_constant: u64,
        input: EsdtTokenPayment<Self::Api>,
    ) -> EsdtTokenPayment<Self::Api> {
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();

        let (token_out, weighted_price) = if input.token_identifier == first_token_id {
            let price = self.compute_first_token_weighted_price(
                first_price_observation,
                last_price_observation,
            );
            (second_token_id, price)
        } else if input.token_identifier == second_token_id {
            let price = self.compute_second_token_weighted_price(
                first_price_observation,
                last_price_observation,
            );
            (first_token_id, price)
        } else {
            sc_panic!(ERROR_UNKNOWN_TOKEN);
        };
        require!(weighted_price > 0u64, ERROR_ZERO_AMOUNT);

        EsdtTokenPayment::new(
            token_out,
            0,
            weighted_price * input.amount / division_safety_constant,
        )
    }

    #[endpoint(updateSafePriceInfo)]
    fn update_safe_price_info(
        &self,
        new_max_observations: usize,
        default_safe_price_rounds_offset: u64,
    ) {
        self.require_caller_has_owner_permissions();
        let safe_price_info_mapper = self.safe_price_info();
        let mut safe_price_info = if !safe_price_info_mapper.is_empty() {
            safe_price_info_mapper.get()
        } else {
            SafePriceInfo {
                current_index: 0,
                max_observations: 0,
                default_safe_price_rounds_offset: 0,
                division_safety_constant: DIVISION_SAFETY_CONSTANT,
            }
        };
        require!(
            new_max_observations >= safe_price_info.max_observations,
            ERROR_SAFE_PRICE_MAX_OBSERVATIONS
        );
        safe_price_info.max_observations = new_max_observations;
        safe_price_info.default_safe_price_rounds_offset = default_safe_price_rounds_offset;
        safe_price_info_mapper.set(safe_price_info);
    }

    fn update_safe_state_on_the_fly(&self) {
        self.update_safe_state(
            &self.pair_reserve(&self.first_token_id().get()).get(),
            &self.pair_reserve(&self.second_token_id().get()).get(),
        );
    }

    fn update_safe_state(&self, first_token_reserve: &BigUint, second_token_reserve: &BigUint) {
        //Skip executing if reserves are 0. This will only happen once, first add_liq after init.
        if first_token_reserve == &0u64 || second_token_reserve == &0u64 {
            return;
        }

        let current_round = self.blockchain().get_block_round();
        let safe_price_info_mapper = self.safe_price_info();
        let mut safe_price_info = safe_price_info_mapper.get();
        let pending_price_observation: PriceObservation<<Self as ContractBase>::Api> =
            if !self.pending_price_observation().is_empty() {
                self.pending_price_observation().get()
            } else {
                PriceObservation::default()
            };
        let price_observations_mapper = self.price_observations();
        let mut price_observations = if !price_observations_mapper.is_empty() {
            price_observations_mapper.get()
        } else {
            ManagedVec::new()
        };

        // Save the previously computed price observation, if it's the last one from the previous block
        if pending_price_observation.recording_round < current_round
            && pending_price_observation.recording_round > 0
        {
            let new_index = if price_observations.len() == 0 {
                0
            } else {
                (safe_price_info.current_index + 1) % safe_price_info.max_observations
            };

            safe_price_info.current_index = new_index;

            if price_observations.len() == safe_price_info.max_observations {
                let _ = price_observations.set(new_index, &pending_price_observation);
            } else {
                price_observations.push(pending_price_observation);
            }

            price_observations_mapper.set(&price_observations);
            safe_price_info_mapper.set(&safe_price_info);
        }

        let mut current_price_observation =
            match price_observations.try_get(safe_price_info.current_index) {
                Some(price_observation) => price_observation,
                None => PriceObservation::default(),
            };

        current_price_observation.compute_new_observation(
            current_round,
            first_token_reserve,
            second_token_reserve,
            safe_price_info.division_safety_constant,
        );

        self.pending_price_observation()
            .set(current_price_observation);
    }

    fn get_price_observation(
        &self,
        current_index: usize,
        max_observations: usize,
        price_observations: &ManagedVec<PriceObservation<Self::Api>>,
        search_round: Round,
    ) -> PriceObservation<Self::Api> {
        if price_observations.len() == 0 {
            sc_panic!(ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST)
        }

        // Check if the requested price observation is the last one
        let last_observation = price_observations.get(current_index);
        if last_observation.recording_round <= search_round {
            return last_observation;
        }

        // Check if the observation round exists in the list
        let oldest_observation_index = (current_index + 1) % max_observations;
        let oldest_observation_option = price_observations.try_get(oldest_observation_index);
        match oldest_observation_option {
            Some(oldest_observation) => {
                if oldest_observation.recording_round == search_round {
                    return oldest_observation;
                } else if oldest_observation.recording_round > search_round {
                    sc_panic!(ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST);
                }
            }
            None => sc_panic!(ERROR_SAFE_PRICE_OBSERVATION_DOES_NOT_EXIST),
        }

        // Binary search algorithm
        let mut search_index = 0;
        let mut left_index;
        let mut right_index;
        let observation_at_index_0 = price_observations.get(0);
        if observation_at_index_0.recording_round <= search_round {
            left_index = 0;
            right_index = current_index - 1;
        } else {
            left_index = current_index + 1;
            right_index = max_observations - 1;
        }

        while left_index <= right_index {
            search_index = (left_index + right_index) / 2;
            let price_observation = price_observations.get(search_index);
            if price_observation.recording_round == search_round {
                return price_observation;
            } else if price_observation.recording_round < search_round {
                left_index = search_index + 1;
            } else {
                right_index = search_index - 1;
            }
        }

        // Linear interpolation in case there is no price observation for the searched round
        let last_found_observation = price_observations.get(search_index);
        let left_observation;
        let right_observation;
        if last_found_observation.recording_round < search_round {
            left_observation = last_found_observation;
            let right_observation_index = (search_index + 1) % max_observations;
            right_observation = price_observations.get(right_observation_index);
        } else {
            let left_observation_index = if search_index == 0 {
                max_observations - 1
            } else {
                search_index - 1
            };
            left_observation = price_observations.get(left_observation_index);
            right_observation = last_found_observation;
        };

        let left_weight = search_round - left_observation.recording_round;
        let right_weight = right_observation.recording_round - search_round;
        let weight_sum = left_weight + right_weight;
        let first_token_price_sum = BigUint::from(left_weight)
            * left_observation.first_token_price_accumulated
            + BigUint::from(right_weight) * right_observation.first_token_price_accumulated;
        let second_token_price_sum = BigUint::from(left_weight)
            * left_observation.second_token_price_accumulated
            + BigUint::from(right_weight) * right_observation.second_token_price_accumulated;

        let first_token_price_accumulated = first_token_price_sum / weight_sum;
        let second_token_price_accumulated = second_token_price_sum / weight_sum;
        let weight_accumulated =
            left_observation.weight_accumulated + search_round - left_observation.recording_round;

        PriceObservation {
            first_token_price_accumulated,
            second_token_price_accumulated,
            weight_accumulated,
            recording_round: search_round,
        }
    }

    fn compute_first_token_weighted_price(
        &self,
        first_price_observation: &PriceObservation<Self::Api>,
        last_price_observation: &PriceObservation<Self::Api>,
    ) -> BigUint {
        let price_diff = last_price_observation.first_token_price_accumulated.clone()
            - first_price_observation
                .first_token_price_accumulated
                .clone();
        let weight_diff = last_price_observation.weight_accumulated.clone()
            - first_price_observation.weight_accumulated.clone();
        price_diff / weight_diff
    }

    fn compute_second_token_weighted_price(
        &self,
        first_price_observation: &PriceObservation<Self::Api>,
        last_price_observation: &PriceObservation<Self::Api>,
    ) -> BigUint {
        let price_diff = last_price_observation
            .second_token_price_accumulated
            .clone()
            - first_price_observation
                .second_token_price_accumulated
                .clone();
        let weight_diff = last_price_observation.weight_accumulated.clone()
            - first_price_observation.weight_accumulated.clone();
        price_diff / weight_diff
    }

    #[view(getPriceObservation)]
    fn get_price_observation_view(&self, search_round: Round) -> PriceObservation<Self::Api> {
        let safe_price_info = self.safe_price_info().get();
        let price_observations = self.price_observations().get();

        self.get_price_observation(
            safe_price_info.current_index,
            safe_price_info.max_observations,
            &price_observations,
            search_round,
        )
    }

    #[view(getPendingPriceObservation)]
    #[storage_mapper("pending_price_observation")]
    fn pending_price_observation(&self) -> SingleValueMapper<PriceObservation<Self::Api>>;

    #[view(getPriceObservations)]
    #[storage_mapper("price_observations")]
    fn price_observations(&self) -> SingleValueMapper<ManagedVec<PriceObservation<Self::Api>>>;

    #[view(getSafePriceInfo)]
    #[storage_mapper("safe_price_info")]
    fn safe_price_info(&self) -> SingleValueMapper<SafePriceInfo>;
}
