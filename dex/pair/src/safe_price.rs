multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use multiversx_sc::codec::{NestedDecodeInput, TopDecodeInput};

use crate::{amm, config, errors::ERROR_SAFE_PRICE_CURRENT_INDEX, read_pair_storage};

pub type Round = u64;
pub type Timestamp = u64;

pub const MAX_OBSERVATIONS: usize = 65_536; // 2^{16} records, to optimise binary search

#[type_abi]
#[derive(ManagedVecItem, Clone, TopEncode, NestedEncode, Debug)]
pub struct PriceObservation<M: ManagedTypeApi> {
    pub first_token_reserve_accumulated: BigUint<M>,
    pub second_token_reserve_accumulated: BigUint<M>,
    pub weight_accumulated: u64,
    pub recording_round: Round,
    pub recording_timestamp: Timestamp,
    pub lp_supply_accumulated: BigUint<M>,
}

impl<M: ManagedTypeApi> Default for PriceObservation<M> {
    fn default() -> Self {
        PriceObservation {
            first_token_reserve_accumulated: BigUint::zero(),
            second_token_reserve_accumulated: BigUint::zero(),
            weight_accumulated: 0,
            recording_round: 0,
            recording_timestamp: 0,
            lp_supply_accumulated: BigUint::zero(),
        }
    }
}

impl<M: ManagedTypeApi> TopDecode for PriceObservation<M> {
    fn top_decode<I>(input: I) -> Result<Self, DecodeError>
    where
        I: TopDecodeInput,
    {
        let mut buffer = input.into_nested_buffer();
        Self::dep_decode(&mut buffer)
    }
}

impl<M: ManagedTypeApi> NestedDecode for PriceObservation<M> {
    fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
        let first_token_reserve_accumulated = BigUint::dep_decode(input)?;
        let second_token_reserve_accumulated = BigUint::dep_decode(input)?;
        let weight_accumulated = u64::dep_decode(input)?;
        let recording_round = u64::dep_decode(input)?;

        let (recording_timestamp, lp_supply_accumulated) = if !input.is_depleted() {
            (u64::dep_decode(input)?, BigUint::dep_decode(input)?)
        } else {
            (0u64, BigUint::zero())
        };

        if !input.is_depleted() {
            return Result::Err(DecodeError::INPUT_TOO_LONG);
        }

        Result::Ok(PriceObservation {
            first_token_reserve_accumulated,
            second_token_reserve_accumulated,
            weight_accumulated,
            recording_round,
            recording_timestamp,
            lp_supply_accumulated,
        })
    }
}

#[multiversx_sc::module]
pub trait SafePriceModule:
    config::ConfigModule
    + read_pair_storage::ReadPairStorageModule
    + token_send::TokenSendModule
    + amm::AmmModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
{
    fn update_safe_price(
        &self,
        first_token_reserve: &BigUint,
        second_token_reserve: &BigUint,
        lp_supply: &BigUint,
    ) {
        if first_token_reserve == &0u64 || second_token_reserve == &0u64 || lp_supply == &0u64 {
            return;
        }

        let current_round = self.blockchain().get_block_round();
        let round_save_interval = self.get_safe_price_round_save_interval();

        // Handle the case where offset is 1 (immediate save)
        if round_save_interval <= 1 {
            self.handle_immediate_save(
                current_round,
                first_token_reserve,
                second_token_reserve,
                lp_supply,
            );
            return;
        }

        // Check if enough rounds have passed since last finalized observation for direct save
        let safe_price_current_index = self.safe_price_current_index().get();
        let last_observation = if safe_price_current_index > 0 {
            self.price_observations().get(safe_price_current_index)
        } else {
            PriceObservation::default()
        };

        let rounds_since_last = current_round - last_observation.recording_round;

        if safe_price_current_index > 0 && rounds_since_last >= round_save_interval {
            self.handle_immediate_save(
                current_round,
                first_token_reserve,
                second_token_reserve,
                lp_supply,
            );
            return;
        }

        // If no current intermediate observation exists, start a new one
        if self.current_price_observation().is_empty() {
            let new_intermediate = self.compute_new_observation(
                current_round,
                first_token_reserve,
                second_token_reserve,
                lp_supply,
                &last_observation,
            );
            self.current_price_observation().set(&new_intermediate);
            return;
        }

        self.update_intermediate_observation(
            current_round,
            first_token_reserve,
            second_token_reserve,
            lp_supply,
        );

        self.save_averaged_observation_if_needed(last_observation.recording_round);
    }

    fn handle_immediate_save(
        &self,
        current_round: Round,
        first_token_reserve: &BigUint,
        second_token_reserve: &BigUint,
        lp_supply: &BigUint,
    ) {
        let safe_price_current_index = self.safe_price_current_index().get();
        let price_observations = self.price_observations();

        let mut last_price_observation = if price_observations.is_empty() {
            PriceObservation::default()
        } else {
            price_observations.get(safe_price_current_index)
        };

        let rounds_since_last_observation = current_round - last_price_observation.recording_round;
        let round_save_interval = self.get_safe_price_round_save_interval();

        if rounds_since_last_observation < round_save_interval {
            return;
        }

        if !self.current_price_observation().is_empty() {
            let current_intermediate = self.current_price_observation().get();
            if current_intermediate.recording_round > last_price_observation.recording_round {
                last_price_observation = current_intermediate;
            }
        }

        let new_price_observation = self.compute_new_observation(
            current_round,
            first_token_reserve,
            second_token_reserve,
            lp_supply,
            &last_price_observation,
        );

        self.save_observation_to_storage(&new_price_observation);
    }

    fn update_intermediate_observation(
        &self,
        current_round: Round,
        first_token_reserve: &BigUint,
        second_token_reserve: &BigUint,
        lp_supply: &BigUint,
    ) {
        let mut current_intermediate = self.current_price_observation().get();
        self.accumulate_into_observation(
            &mut current_intermediate,
            current_round,
            first_token_reserve,
            second_token_reserve,
            lp_supply,
        );
        self.current_price_observation().set(&current_intermediate);
    }

    fn save_observation_to_storage(&self, price_observation: &PriceObservation<Self::Api>) {
        let safe_price_current_index = self.safe_price_current_index().get();
        require!(
            safe_price_current_index <= MAX_OBSERVATIONS,
            ERROR_SAFE_PRICE_CURRENT_INDEX
        );

        let mut price_observations = self.price_observations();

        let new_index = if price_observations.is_empty() {
            1
        } else {
            (safe_price_current_index % MAX_OBSERVATIONS) + 1
        };

        if price_observations.len() == MAX_OBSERVATIONS {
            price_observations.set(new_index, price_observation);
        } else {
            price_observations.push(price_observation);
        }

        self.safe_price_current_index().set(new_index);
        self.current_price_observation().clear();
    }

    fn save_averaged_observation_if_needed(&self, last_observation_round: Round) {
        let current_intermediate = self.current_price_observation().get();
        let round_save_interval = self.get_safe_price_round_save_interval();

        if last_observation_round == 0
            && current_intermediate.weight_accumulated < round_save_interval
        {
            return;
        }

        if current_intermediate.recording_round - last_observation_round < round_save_interval {
            return;
        }

        self.save_observation_to_storage(&current_intermediate);
    }

    fn accumulate_into_observation(
        &self,
        observation: &mut PriceObservation<Self::Api>,
        current_round: Round,
        first_token_reserve: &BigUint,
        second_token_reserve: &BigUint,
        lp_supply: &BigUint,
    ) {
        let mut weight = 1;
        if observation.recording_round > 0 {
            weight = current_round - observation.recording_round;
        }

        observation.first_token_reserve_accumulated += BigUint::from(weight) * first_token_reserve;
        observation.second_token_reserve_accumulated +=
            BigUint::from(weight) * second_token_reserve;
        observation.lp_supply_accumulated += BigUint::from(weight) * lp_supply;
        observation.weight_accumulated += weight;
        observation.recording_round = current_round;
        observation.recording_timestamp = self.blockchain().get_block_timestamp();
    }

    fn compute_new_observation(
        &self,
        new_round: Round,
        new_first_reserve: &BigUint,
        new_second_reserve: &BigUint,
        new_lp_supply: &BigUint,
        current_price_observation: &PriceObservation<Self::Api>,
    ) -> PriceObservation<Self::Api> {
        let mut new_price_observation = current_price_observation.clone();
        self.accumulate_into_observation(
            &mut new_price_observation,
            new_round,
            new_first_reserve,
            new_second_reserve,
            new_lp_supply,
        );
        new_price_observation
    }

    fn get_safe_price_round_save_interval(&self) -> Round {
        let router_address = self.router_address().get();
        let safe_price_round_save_interval = self
            .get_safe_price_round_save_interval_mapper(router_address)
            .get();
        require!(
            safe_price_round_save_interval > 0,
            "Safe price round save interval not set"
        );
        safe_price_round_save_interval
    }

    #[storage_mapper("price_observations")]
    fn price_observations(&self) -> VecMapper<PriceObservation<Self::Api>>;

    #[view(getSafePriceCurrentIndex)]
    #[storage_mapper("safe_price_current_index")]
    fn safe_price_current_index(&self) -> SingleValueMapper<usize>;

    #[view(getCurrentPriceObservation)]
    #[storage_mapper("current_price_observation")]
    fn current_price_observation(&self) -> SingleValueMapper<PriceObservation<Self::Api>>;
}
