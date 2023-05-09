multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    amm, config,
    errors::{ERROR_SAFE_PRICE_MAX_OBSERVATIONS, ERROR_SAFE_PRICE_NEW_MAX_OBSERVATIONS},
};

pub type Round = u64;

#[derive(Clone, TopEncode, TopDecode, TypeAbi)]
pub struct SafePriceParams {
    pub current_index: usize,
    pub max_observations: usize,
}

#[derive(ManagedVecItem, Clone, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi)]
pub struct PriceObservation<M: ManagedTypeApi> {
    pub first_token_reserve_accumulated: BigUint<M>,
    pub second_token_reserve_accumulated: BigUint<M>,
    pub weight_accumulated: u64,
    pub recording_round: Round,
}

impl<M: ManagedTypeApi> Default for PriceObservation<M> {
    fn default() -> Self {
        PriceObservation {
            first_token_reserve_accumulated: BigUint::zero(),
            second_token_reserve_accumulated: BigUint::zero(),
            weight_accumulated: 0,
            recording_round: 0,
        }
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
    fn update_safe_price(&self, first_token_reserve: &BigUint, second_token_reserve: &BigUint) {
        //Skip executing if reserves are 0. This will only happen once, first add_liq after init.
        if first_token_reserve == &0u64 || second_token_reserve == &0u64 {
            return;
        }

        let current_round = self.blockchain().get_block_round();
        let safe_price_max_observations = self.safe_price_max_observations().get();
        let safe_price_current_index = self.safe_price_current_index().get();
        require!(
            safe_price_max_observations > 0,
            ERROR_SAFE_PRICE_MAX_OBSERVATIONS
        );

        let price_observations_mapper = self.price_observations();

        let mut price_observations = ManagedVec::new();
        let mut last_price_observation = PriceObservation::default();
        let mut new_index = 0;
        if !price_observations_mapper.is_empty() {
            price_observations = price_observations_mapper.get();
            last_price_observation = price_observations.get(safe_price_current_index);
            new_index = (safe_price_current_index + 1) % safe_price_max_observations;
        }

        if last_price_observation.recording_round == current_round {
            return;
        }

        let new_price_observation = self.compute_new_observation(
            current_round,
            first_token_reserve,
            second_token_reserve,
            &last_price_observation,
        );

        if price_observations.len() == safe_price_max_observations {
            let _ = price_observations.set(new_index, &new_price_observation);
        } else {
            price_observations.push(new_price_observation);
        }

        price_observations_mapper.set(&price_observations);
        self.safe_price_current_index().set(new_index);
    }

    fn compute_new_observation(
        &self,
        new_round: Round,
        new_first_reserve: &BigUint,
        new_second_reserve: &BigUint,
        current_price_observation: &PriceObservation<Self::Api>,
    ) -> PriceObservation<Self::Api> {
        let new_weight = if current_price_observation.recording_round > 0 {
            new_round - current_price_observation.recording_round
        } else {
            1
        };

        // Create a new variable, to avoid overwriting the old price observation
        let mut new_price_observation = current_price_observation.clone();
        new_price_observation.first_token_reserve_accumulated +=
            BigUint::from(new_weight) * new_first_reserve;
        new_price_observation.second_token_reserve_accumulated +=
            BigUint::from(new_weight) * new_second_reserve;
        new_price_observation.weight_accumulated += new_weight;
        new_price_observation.recording_round = new_round;

        new_price_observation
    }

    #[endpoint(setSafePriceMaxObservations)]
    fn set_safe_price_max_observations(&self, new_max_observations: usize) {
        self.require_caller_has_owner_permissions();
        self.safe_price_max_observations()
            .update(|max_observations| {
                require!(
                    &new_max_observations >= max_observations && new_max_observations > 0,
                    ERROR_SAFE_PRICE_NEW_MAX_OBSERVATIONS
                );
                *max_observations = new_max_observations
            });
    }

    #[view(getPriceObservations)]
    #[storage_mapper("price_observations")]
    fn price_observations(&self) -> SingleValueMapper<ManagedVec<PriceObservation<Self::Api>>>;

    #[view(getSafePriceCurrentIndex)]
    #[storage_mapper("safe_price_current_index")]
    fn safe_price_current_index(&self) -> SingleValueMapper<usize>;

    #[view(getSafePriceMaxObservations)]
    #[storage_mapper("safe_price_max_observations")]
    fn safe_price_max_observations(&self) -> SingleValueMapper<usize>;
}
