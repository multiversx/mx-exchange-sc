multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    amm, config,
    errors::{ERROR_SAFE_PRICE_MAX_OBSERVATIONS, ERROR_SAFE_PRICE_NEW_MAX_OBSERVATIONS},
};

pub type Round = u64;

#[derive(Clone, TopEncode, TopDecode, TypeAbi)]
pub struct SafePriceInfo {
    pub current_index: usize,
    pub max_observations: usize,
    pub default_safe_price_rounds_offset: u64,
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

impl<M: ManagedTypeApi> PriceObservation<M> {
    fn compute_new_observation(
        &mut self,
        new_round: Round,
        new_first_reserve: &BigUint<M>,
        new_second_reserve: &BigUint<M>,
    ) {
        let new_weight = new_round - self.recording_round;
        self.first_token_reserve_accumulated += BigUint::from(new_weight) * new_first_reserve;
        self.second_token_reserve_accumulated += BigUint::from(new_weight) * new_second_reserve;
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
            }
        };
        require!(
            new_max_observations >= safe_price_info.max_observations,
            ERROR_SAFE_PRICE_NEW_MAX_OBSERVATIONS
        );
        safe_price_info.max_observations = new_max_observations;
        safe_price_info.default_safe_price_rounds_offset = default_safe_price_rounds_offset;
        safe_price_info_mapper.set(safe_price_info);
    }

    fn update_safe_price(&self, first_token_reserve: &BigUint, second_token_reserve: &BigUint) {
        //Skip executing if reserves are 0. This will only happen once, first add_liq after init.
        if first_token_reserve == &0u64 || second_token_reserve == &0u64 {
            return;
        }

        let current_round = self.blockchain().get_block_round();
        let safe_price_info_mapper = self.safe_price_info();
        let mut safe_price_info = safe_price_info_mapper.get();

        require!(
            safe_price_info.max_observations > 0,
            ERROR_SAFE_PRICE_MAX_OBSERVATIONS
        );

        let pending_price_observation_mapper = self.pending_price_observation();
        let pending_price_observation: PriceObservation<<Self as ContractBase>::Api> =
            if !pending_price_observation_mapper.is_empty() {
                pending_price_observation_mapper.get()
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
            let new_index = if price_observations.is_empty() {
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

        let mut current_price_observation = if price_observations_mapper.is_empty() {
            PriceObservation::default()
        } else {
            price_observations.get(safe_price_info.current_index)
        };

        current_price_observation.compute_new_observation(
            current_round,
            first_token_reserve,
            second_token_reserve,
        );

        pending_price_observation_mapper.set(current_price_observation);
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
