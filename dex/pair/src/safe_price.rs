multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use multiversx_sc::codec::{NestedDecodeInput, TopDecodeInput};

use crate::{
    amm, config,
    errors::{
        ERROR_SAFE_PRICE_CURRENT_INDEX, ERROR_SAFE_PRICE_DURATION_OVERFLOW,
        ERROR_SAFE_PRICE_LEGACY_NORMALIZATION,
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
        if current_timestamp == 0 {
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
        if last_recorded_observation.recording_round > 0
            && last_recorded_observation.recording_timestamp == 0
        {
            let legacy_cutover_mapper = self.safe_price_legacy_cutover();
            require!(
                !legacy_cutover_mapper.is_empty(),
                ERROR_SAFE_PRICE_LEGACY_NORMALIZATION
            );
            let legacy_cutover = legacy_cutover_mapper.get();
            require!(
                self.normalize_legacy_observation(
                    &mut last_recorded_observation,
                    legacy_cutover,
                ),
                ERROR_SAFE_PRICE_LEGACY_NORMALIZATION
            );
        }

        let last_recorded_weight = last_recorded_observation.weight_accumulated;
        let current_price_observation_mapper = self.current_price_observation();
        let mut latest_observation = if current_price_observation_mapper.is_empty() {
            last_recorded_observation
        } else {
            current_price_observation_mapper.get()
        };

        if latest_observation.weight_accumulated > 0
            && latest_observation.recording_timestamp >= current_timestamp
        {
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

        let timestamp_save_interval = self.get_safe_price_timestamp_save_interval();
        if latest_observation.weight_accumulated - last_recorded_weight >= timestamp_save_interval
        {
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

    fn normalize_legacy_observation(
        &self,
        observation: &mut PriceObservation<Self::Api>,
        legacy_cutover: (Round, Timestamp),
    ) -> bool {
        let (cutover_round, cutover_timestamp) = legacy_cutover;
        if observation.recording_round > cutover_round {
            return false;
        }

        let elapsed_rounds = cutover_round - observation.recording_round;
        if elapsed_rounds > u64::MAX / LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS {
            return false;
        }

        let elapsed_milliseconds =
            elapsed_rounds * LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS;
        if elapsed_milliseconds >= cutover_timestamp {
            return false;
        }

        observation.recording_timestamp = cutover_timestamp - elapsed_milliseconds;
        self.scale_legacy_observation_to_milliseconds(observation);
        true
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
            LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS
        };
        let weight_biguint = BigUint::from(weight);

        observation.first_token_reserve_accumulated += &weight_biguint * first_token_reserve;
        observation.second_token_reserve_accumulated += &weight_biguint * second_token_reserve;
        observation.lp_supply_accumulated += weight_biguint * lp_supply;
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

    fn scale_legacy_observation_to_milliseconds(
        &self,
        observation: &mut PriceObservation<Self::Api>,
    ) {
        require!(
            observation.weight_accumulated
                <= u64::MAX / LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS,
            ERROR_SAFE_PRICE_DURATION_OVERFLOW
        );

        let multiplier = BigUint::from(LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS);
        observation.first_token_reserve_accumulated *= &multiplier;
        observation.second_token_reserve_accumulated *= &multiplier;
        observation.lp_supply_accumulated *= &multiplier;
        observation.weight_accumulated *= LEGACY_SAFE_PRICE_ROUND_DURATION_MILLISECONDS;
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
