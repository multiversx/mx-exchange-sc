elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const PRICE_DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
const OBSERVATION_FREQUENCY_BLOCKS: u64 = 600;
const OBSERVATIONS_MAX_LEN: usize = 10_000;

type Nonce = u64;

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct PriceObservation<BigUint: BigUintApi> {
    first_token_price: BigUint,
    second_token_price: BigUint,
    observation_block: Nonce,
}

#[elrond_wasm_derive::module]
pub trait PricesModule {
    fn update_price_observation(
        &self,
        first_token_reserve: &Self::BigUint,
        second_token_reserve: &Self::BigUint,
    ) -> SCResult<()> {
        require!(first_token_reserve > &0, "First token reserve is zero");
        require!(second_token_reserve > &0, "Second token reserve is zero");

        let current_block = self.blockchain().get_block_nonce();
        let last_index = self.last_price_observation_index().get();
        let default_value_fn = || PriceObservation::<Self::BigUint> {
            first_token_price: Self::BigUint::zero(),
            second_token_price: Self::BigUint::zero(),
            observation_block: current_block,
        };
        let last_price_obs = self
            .price_observations()
            .get_or_else(last_index, default_value_fn);

        if self.should_commit_current_price(current_block, last_price_obs.observation_block) {
            self.commit_current_price(current_block, last_index);
            self.reset_current_price(first_token_reserve, second_token_reserve);
            self.last_price_update_block().set(&current_block);
        } else {
            let last_price_update_block = self.last_price_update_block().get();

            if self.should_update_current_price(current_block, last_price_update_block) {
                self.update_current_price(
                    current_block,
                    last_price_update_block,
                    last_price_obs.observation_block,
                    first_token_reserve,
                    second_token_reserve,
                );
                self.last_price_update_block().set(&current_block);
            }
        }

        Ok(())
    }

    fn should_commit_current_price(&self, current_block: Nonce, last_obs_block: Nonce) -> bool {
        current_block > last_obs_block + OBSERVATION_FREQUENCY_BLOCKS
    }

    fn should_update_current_price(
        &self,
        current_block: Nonce,
        last_price_update_block: Nonce,
    ) -> bool {
        current_block > last_price_update_block
    }

    fn commit_current_price(&self, current_block: Nonce, last_obs_index: usize) {
        let observation = PriceObservation::<Self::BigUint> {
            first_token_price: self.first_token_price().get(),
            second_token_price: self.second_token_price().get(),
            observation_block: current_block,
        };

        let len = self.price_observations().len();
        if len < OBSERVATIONS_MAX_LEN {
            self.price_observations().push(&observation);
            self.last_price_observation_index().set(&len);
        } else {
            let new_obs_index = (last_obs_index + 1) % OBSERVATIONS_MAX_LEN;
            self.price_observations().set(new_obs_index, &observation);
            self.last_price_observation_index().set(&new_obs_index);
        }
    }

    fn update_current_price(
        &self,
        current_block: Nonce,
        last_price_update_block: Nonce,
        last_obs_block: Nonce,
        first_token_reserve: &Self::BigUint,
        second_token_reserve: &Self::BigUint,
    ) {
        let instant_first_token_price =
            self.instant_price(second_token_reserve, first_token_reserve);
        let instant_second_token_price =
            self.instant_price(first_token_reserve, second_token_reserve);

        let first_token_price = self.first_token_price().get();
        let second_token_price = self.second_token_price().get();

        let instant_price_period = current_block - last_price_update_block;
        let price_period = last_price_update_block - last_obs_block;

        let weighted_first_token_price = self.calculate_weighted_price(
            first_token_price,
            price_period,
            instant_first_token_price,
            instant_price_period,
        );
        let weighted_second_token_price = self.calculate_weighted_price(
            second_token_price,
            price_period,
            instant_second_token_price,
            instant_price_period,
        );

        self.first_token_price().set(&weighted_first_token_price);
        self.second_token_price().set(&weighted_second_token_price);
    }

    fn reset_current_price(
        &self,
        first_token_reserve: &Self::BigUint,
        second_token_reserve: &Self::BigUint,
    ) {
        self.first_token_price()
            .set(&self.instant_price(second_token_reserve, first_token_reserve));
        self.second_token_price()
            .set(&self.instant_price(first_token_reserve, second_token_reserve));
    }

    fn instant_price(
        &self,
        numerator: &Self::BigUint,
        denominator: &Self::BigUint,
    ) -> Self::BigUint {
        numerator * &Self::BigUint::from(PRICE_DIVISION_SAFETY_CONSTANT) / denominator.clone()
    }

    fn calculate_weighted_price(
        &self,
        weight_price: Self::BigUint,
        weight_price_period: u64,
        instant_price: Self::BigUint,
        instant_price_period: u64,
    ) -> Self::BigUint {
        (weight_price * Self::BigUint::from(weight_price_period)
            + instant_price * Self::BigUint::from(instant_price_period))
            / Self::BigUint::from(weight_price_period + instant_price_period)
    }

    #[view(getFirstTokenPrice)]
    #[storage_mapper("first_token_price")]
    fn first_token_price(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[view(getSecondTokenPrice)]
    #[storage_mapper("second_token_price")]
    fn second_token_price(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[view(getLastPriceUpdateBlock)]
    #[storage_mapper("last_price_update_block")]
    fn last_price_update_block(&self) -> SingleValueMapper<Self::Storage, Nonce>;

    #[view(getPriceObservations)]
    #[storage_mapper("price_observations")]
    fn price_observations(&self) -> VecMapper<Self::Storage, PriceObservation<Self::BigUint>>;

    #[view(getLastPriceObservationIndex)]
    #[storage_mapper("last_price_observation_index")]
    fn last_price_observation_index(&self) -> SingleValueMapper<Self::Storage, usize>;
}
