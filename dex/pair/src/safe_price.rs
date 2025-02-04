multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use multiversx_sc::codec::{NestedDecodeInput, TopDecodeInput};

use crate::{amm, config, errors::ERROR_SAFE_PRICE_CURRENT_INDEX};

pub type Round = u64;
pub type Timestamp = u64;

pub const DEFAULT_ROUND_SAVE_INTERVAL: u64 = 1;
pub const MAX_OBSERVATIONS: usize = 65_536; // 2^{16} records, to optimise binary search

#[derive(ManagedVecItem, Clone, TopEncode, NestedEncode, TypeAbi, Debug)]
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
        let safe_price_current_index = self.safe_price_current_index().get();
        require!(
            safe_price_current_index <= MAX_OBSERVATIONS,
            ERROR_SAFE_PRICE_CURRENT_INDEX
        );

        let mut price_observations = self.price_observations();
        let mut last_price_observation = PriceObservation::default();
        let mut new_index = 1;
        if !price_observations.is_empty() {
            last_price_observation = price_observations.get(safe_price_current_index);
            new_index = (safe_price_current_index % MAX_OBSERVATIONS) + 1;
        }

        let rounds_since_last_observation = current_round - last_price_observation.recording_round;
        let safe_price_round_save_interval_mapper = self.safe_price_round_save_interval();
        let round_save_interval = match safe_price_round_save_interval_mapper.get() {
            0 => DEFAULT_ROUND_SAVE_INTERVAL,
            value => value,
        };

        if rounds_since_last_observation < round_save_interval {
            return;
        }

        let new_price_observation = self.compute_new_observation(
            current_round,
            first_token_reserve,
            second_token_reserve,
            lp_supply,
            &last_price_observation,
        );

        if price_observations.len() == MAX_OBSERVATIONS {
            price_observations.set(new_index, &new_price_observation);
        } else {
            price_observations.push(&new_price_observation);
        }

        self.safe_price_current_index().set(new_index);
    }

    fn compute_new_observation(
        &self,
        new_round: Round,
        new_first_reserve: &BigUint,
        new_second_reserve: &BigUint,
        new_lp_supply: &BigUint,
        current_price_observation: &PriceObservation<Self::Api>,
    ) -> PriceObservation<Self::Api> {
        let new_weight = if current_price_observation.recording_round == 0 {
            1
        } else {
            new_round - current_price_observation.recording_round
        };

        let mut new_price_observation = current_price_observation.clone();
        new_price_observation.first_token_reserve_accumulated +=
            BigUint::from(new_weight) * new_first_reserve;
        new_price_observation.second_token_reserve_accumulated +=
            BigUint::from(new_weight) * new_second_reserve;
        new_price_observation.lp_supply_accumulated += BigUint::from(new_weight) * new_lp_supply;
        new_price_observation.weight_accumulated += new_weight;
        new_price_observation.recording_round = new_round;
        new_price_observation.recording_timestamp = self.blockchain().get_block_timestamp();

        new_price_observation
    }

    #[only_owner]
    #[endpoint(setSafePriceRoundSaveInterval)]
    fn set_safe_price_round_save_interval(&self, new_interval: u64) {
        require!(
            new_interval > 0,
            "Round save interval must be greater than 0"
        );
        self.safe_price_round_save_interval().set(new_interval);
    }

    #[storage_mapper("price_observations")]
    fn price_observations(&self) -> VecMapper<PriceObservation<Self::Api>>;

    #[view(getSafePriceCurrentIndex)]
    #[storage_mapper("safe_price_current_index")]
    fn safe_price_current_index(&self) -> SingleValueMapper<usize>;

    #[view(getSafePriceRoundSaveInterval)]
    #[storage_mapper("safe_price_round_save_interval")]
    fn safe_price_round_save_interval(&self) -> SingleValueMapper<u64>;
}
