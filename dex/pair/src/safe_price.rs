multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use multiversx_sc::codec::{NestedDecodeInput, TopDecodeInput};

use crate::{amm, config, errors::ERROR_SAFE_PRICE_CURRENT_INDEX, read_pair_storage};

pub type Round = u64;
pub type Timestamp = u64;

pub const MAX_OBSERVATIONS: usize = 65_536; // 2^{16} records, to optimise binary search
pub const DEFAULT_SAFE_PRICE_TIMESTAMP_SAVE_INTERVAL_MILLISECONDS: u64 = 6_000;

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
        let current_timestamp = self.get_current_timestamp_milliseconds();
        let timestamp_save_interval = self.get_safe_price_timestamp_save_interval();
        let safe_price_current_index = self.safe_price_current_index().get();
        require!(
            safe_price_current_index <= MAX_OBSERVATIONS,
            ERROR_SAFE_PRICE_CURRENT_INDEX
        );

        let mut last_recorded_observation = if safe_price_current_index > 0 {
            self.price_observations().get(safe_price_current_index)
        } else {
            PriceObservation::default()
        };
        self.prepare_observation_for_timestamp_weights(
            &mut last_recorded_observation,
            current_round,
            current_timestamp,
        );
        let latest_observation = self.get_latest_price_observation(
            &last_recorded_observation,
            current_round,
            current_timestamp,
        );

        if latest_observation.weight_accumulated > 0
            && (latest_observation.recording_round == current_round
                || latest_observation.recording_timestamp >= current_timestamp)
        {
            return;
        }

        let new_price_observation = self.compute_new_observation(
            current_round,
            current_timestamp,
            first_token_reserve,
            second_token_reserve,
            lp_supply,
            &latest_observation,
        );

        self.current_price_observation().set(&new_price_observation);

        let should_save_observation = new_price_observation.weight_accumulated
            - last_recorded_observation.weight_accumulated
            >= timestamp_save_interval;

        if should_save_observation {
            self.save_observation_to_storage(&new_price_observation);
        }
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
    }

    fn get_latest_price_observation(
        &self,
        last_recorded_observation: &PriceObservation<Self::Api>,
        current_round: Round,
        current_timestamp: Timestamp,
    ) -> PriceObservation<Self::Api> {
        if self.current_price_observation().is_empty() {
            return last_recorded_observation.clone();
        }

        let mut current_price_observation = self.current_price_observation().get();
        self.prepare_observation_for_timestamp_weights(
            &mut current_price_observation,
            current_round,
            current_timestamp,
        );
        if current_price_observation.recording_round > last_recorded_observation.recording_round {
            current_price_observation
        } else {
            last_recorded_observation.clone()
        }
    }

    fn prepare_observation_for_timestamp_weights(
        &self,
        observation: &mut PriceObservation<Self::Api>,
        current_round: Round,
        current_timestamp: Timestamp,
    ) {
        if observation.recording_round == 0 || observation.recording_timestamp > 0 {
            return;
        }

        let elapsed_rounds = current_round.saturating_sub(observation.recording_round);
        let elapsed_milliseconds = self.legacy_weight_to_milliseconds(elapsed_rounds);
        observation.recording_timestamp = current_timestamp.saturating_sub(elapsed_milliseconds);

        let multiplier = BigUint::from(DEFAULT_SAFE_PRICE_TIMESTAMP_SAVE_INTERVAL_MILLISECONDS);
        observation.first_token_reserve_accumulated *= &multiplier;
        observation.second_token_reserve_accumulated *= &multiplier;
        observation.lp_supply_accumulated *= &multiplier;
        observation.weight_accumulated =
            self.legacy_weight_to_milliseconds(observation.weight_accumulated);
    }

    fn accumulate_into_observation(
        &self,
        observation: &mut PriceObservation<Self::Api>,
        current_round: Round,
        current_timestamp: Timestamp,
        first_token_reserve: &BigUint,
        second_token_reserve: &BigUint,
        lp_supply: &BigUint,
    ) {
        let weight = if observation.recording_timestamp > 0 {
            current_timestamp - observation.recording_timestamp
        } else {
            DEFAULT_SAFE_PRICE_TIMESTAMP_SAVE_INTERVAL_MILLISECONDS
        };

        observation.first_token_reserve_accumulated += BigUint::from(weight) * first_token_reserve;
        observation.second_token_reserve_accumulated +=
            BigUint::from(weight) * second_token_reserve;
        observation.lp_supply_accumulated += BigUint::from(weight) * lp_supply;
        observation.weight_accumulated += weight;
        observation.recording_round = current_round;
        observation.recording_timestamp = current_timestamp;
    }

    fn compute_new_observation(
        &self,
        new_round: Round,
        new_timestamp: Timestamp,
        new_first_reserve: &BigUint,
        new_second_reserve: &BigUint,
        new_lp_supply: &BigUint,
        current_price_observation: &PriceObservation<Self::Api>,
    ) -> PriceObservation<Self::Api> {
        let mut new_price_observation = current_price_observation.clone();
        self.accumulate_into_observation(
            &mut new_price_observation,
            new_round,
            new_timestamp,
            new_first_reserve,
            new_second_reserve,
            new_lp_supply,
        );
        new_price_observation
    }

    fn get_current_timestamp_milliseconds(&self) -> Timestamp {
        self.blockchain()
            .get_block_timestamp_millis()
            .as_u64_millis()
    }

    fn legacy_weight_to_milliseconds(&self, legacy_weight: u64) -> u64 {
        match legacy_weight.checked_mul(DEFAULT_SAFE_PRICE_TIMESTAMP_SAVE_INTERVAL_MILLISECONDS) {
            Some(value) => value,
            None => sc_panic!("Safe price duration overflow"),
        }
    }

    fn get_safe_price_timestamp_save_interval(&self) -> Timestamp {
        let router_address = self.router_address().get();
        let safe_price_timestamp_save_interval = self
            .get_safe_price_timestamp_save_interval_mapper(router_address)
            .get();
        require!(
            safe_price_timestamp_save_interval > 0,
            "Safe price timestamp save interval not set"
        );
        safe_price_timestamp_save_interval
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
