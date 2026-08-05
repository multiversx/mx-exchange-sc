multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use multiversx_sc::codec::{NestedDecodeInput, TopDecodeInput};

use crate::{
    amm, config,
    errors::{
        ERROR_SAFE_PRICE_CURRENT_INDEX, ERROR_SAFE_PRICE_LEGACY_NORMALIZATION,
        ERROR_SAFE_PRICE_TIMESTAMP_ORDER,
    },
    read_pair_storage,
};

pub type Round = u64;
pub type Timestamp = u64;

pub const MAX_OBSERVATIONS: usize = 65_536; // 2^{16} records, to optimise binary search
pub const LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS: u64 = 6_000;

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

        let (recording_timestamp, lp_supply_accumulated) = if input.is_depleted() {
            (0u64, BigUint::zero())
        } else {
            (u64::dep_decode(input)?, BigUint::dep_decode(input)?)
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
        if current_round == 0 || current_timestamp == 0 {
            return;
        }

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
        if safe_price_current_index > 0 {
            self.normalize_observation_if_legacy(&mut last_recorded_observation);
        }

        let last_recorded_timestamp = last_recorded_observation.recording_timestamp;
        let current_price_observation_mapper = self.current_price_observation();
        let mut latest_observation =
            if safe_price_current_index == 0 && current_price_observation_mapper.is_empty() {
                PriceObservation::default()
            } else {
                current_price_observation_mapper.get()
            };

        if safe_price_current_index > 0 {
            require!(
                latest_observation.recording_timestamp >= last_recorded_timestamp,
                ERROR_SAFE_PRICE_TIMESTAMP_ORDER
            );
        }
        if latest_observation.recording_timestamp == 0
            && self.get_current_round_duration_milliseconds() == 0
        {
            return;
        }
        if latest_observation.recording_timestamp >= current_timestamp {
            return;
        }

        self.accumulate_into_observation(
            &mut latest_observation,
            current_round,
            current_timestamp,
            first_token_reserve,
            second_token_reserve,
            lp_supply,
        );

        let elapsed_since_last_saved_observation = if safe_price_current_index == 0 {
            latest_observation.weight_accumulated
        } else {
            latest_observation.recording_timestamp - last_recorded_timestamp
        };

        if elapsed_since_last_saved_observation >= self.get_safe_price_timestamp_save_interval() {
            self.save_observation_to_storage(&latest_observation, safe_price_current_index);
        } else {
            current_price_observation_mapper.set(&latest_observation);
        }
    }

    fn save_observation_to_storage(
        &self,
        price_observation: &PriceObservation<Self::Api>,
        safe_price_current_index: usize,
    ) {
        let mut price_observations = self.price_observations();
        let observation_count = price_observations.len();

        let new_index = if observation_count == 0 {
            1
        } else {
            (safe_price_current_index % MAX_OBSERVATIONS) + 1
        };

        if observation_count == MAX_OBSERVATIONS {
            price_observations.set(new_index, price_observation);
        } else {
            price_observations.push(price_observation);
        }

        self.safe_price_current_index().set(new_index);
        self.current_price_observation().set(price_observation);
    }

    fn initialize_current_price_observation(&self) {
        let current_price_observation_mapper = self.current_price_observation();
        if !current_price_observation_mapper.is_empty() {
            return;
        }

        let safe_price_current_index = self.safe_price_current_index().get();
        let price_observations = self.price_observations();
        if safe_price_current_index == 0 {
            require!(
                price_observations.is_empty(),
                ERROR_SAFE_PRICE_CURRENT_INDEX
            );
            return;
        }
        require!(
            safe_price_current_index <= MAX_OBSERVATIONS
                && safe_price_current_index <= price_observations.len(),
            ERROR_SAFE_PRICE_CURRENT_INDEX
        );

        let mut current_price_observation = price_observations.get(safe_price_current_index);
        self.normalize_observation_if_legacy(&mut current_price_observation);

        current_price_observation_mapper.set(&current_price_observation);
    }

    fn normalize_observation_if_legacy(&self, observation: &mut PriceObservation<Self::Api>) {
        if observation.recording_timestamp > 0 {
            return;
        }

        let legacy_cutover_mapper = self.safe_price_legacy_cutover();
        require!(
            !legacy_cutover_mapper.is_empty(),
            ERROR_SAFE_PRICE_LEGACY_NORMALIZATION
        );
        self.normalize_legacy_observation(observation, legacy_cutover_mapper.get());
    }

    fn normalize_legacy_observation(
        &self,
        observation: &mut PriceObservation<Self::Api>,
        legacy_cutover: (Round, Timestamp),
    ) {
        require!(
            observation.lp_supply_accumulated == 0u64,
            ERROR_SAFE_PRICE_LEGACY_NORMALIZATION
        );

        let (cutover_round, cutover_timestamp) = legacy_cutover;
        require!(
            observation.recording_round <= cutover_round,
            ERROR_SAFE_PRICE_LEGACY_NORMALIZATION
        );

        let elapsed_rounds = cutover_round - observation.recording_round;
        let elapsed_milliseconds = elapsed_rounds * LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS;
        require!(
            elapsed_milliseconds < cutover_timestamp,
            ERROR_SAFE_PRICE_LEGACY_NORMALIZATION
        );

        observation.recording_timestamp = cutover_timestamp - elapsed_milliseconds;

        let multiplier = BigUint::from(LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS);
        observation.first_token_reserve_accumulated *= &multiplier;
        observation.second_token_reserve_accumulated *= &multiplier;
        observation.weight_accumulated *= LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS;
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
            self.get_current_round_duration_milliseconds()
        };
        let weight_biguint = BigUint::from(weight);

        observation.first_token_reserve_accumulated += &weight_biguint * first_token_reserve;
        observation.second_token_reserve_accumulated += &weight_biguint * second_token_reserve;
        observation.lp_supply_accumulated += weight_biguint * lp_supply;
        observation.weight_accumulated += weight;
        observation.recording_round = current_round;
        observation.recording_timestamp = current_timestamp;
    }

    fn get_current_timestamp_milliseconds(&self) -> Timestamp {
        self.blockchain()
            .get_block_timestamp_millis()
            .as_u64_millis()
    }

    fn get_current_round_duration_milliseconds(&self) -> Timestamp {
        self.blockchain()
            .get_block_round_time_millis()
            .as_u64_millis()
    }

    fn get_safe_price_timestamp_save_interval(&self) -> Timestamp {
        let safe_price_timestamp_save_interval = self
            .get_safe_price_timestamp_save_interval_mapper(self.router_address().get())
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

    #[storage_mapper("safe_price_legacy_cutover")]
    fn safe_price_legacy_cutover(&self) -> SingleValueMapper<(Round, Timestamp)>;
}
