elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::config;
use crate::{
    amm,
    contexts::base::Context,
    errors::{ERROR_UNKNOWN_TOKEN, ERROR_ZERO_AMOUNT},
};

const MAX_OBSERVATIONS_PER_RECORD: u64 = 10;

type Block = u64;

#[derive(Clone, TopEncode, TopDecode)]
pub struct CumulativeState<M: ManagedTypeApi> {
    pub first_obs_block: Block,
    pub last_obs_block: Block,
    pub num_observations: u64,
    pub first_token_reserve_last_obs: BigUint<M>,
    pub second_token_reserve_last_obs: BigUint<M>,
    pub first_token_reserve_weighted: BigUint<M>,
    pub second_token_reserve_weighted: BigUint<M>,
}

impl<M: ManagedTypeApi> Default for CumulativeState<M> {
    fn default() -> Self {
        CumulativeState {
            first_obs_block: 0,
            last_obs_block: 0,
            num_observations: 0,
            first_token_reserve_last_obs: BigUint::zero(),
            second_token_reserve_last_obs: BigUint::zero(),
            first_token_reserve_weighted: BigUint::zero(),
            second_token_reserve_weighted: BigUint::zero(),
        }
    }
}

impl<M: ManagedTypeApi> CumulativeState<M> {
    fn new(block: u64, first_reserve: &BigUint<M>, second_reserve: &BigUint<M>) -> Self {
        CumulativeState {
            first_obs_block: block,
            last_obs_block: block,
            num_observations: 0,
            first_token_reserve_last_obs: first_reserve.clone(),
            second_token_reserve_last_obs: second_reserve.clone(),
            first_token_reserve_weighted: first_reserve.clone(),
            second_token_reserve_weighted: second_reserve.clone(),
        }
    }

    fn contains_block(&self, block: u64) -> bool {
        self.first_obs_block <= block && block <= self.last_obs_block
    }

    fn is_default(&self) -> bool {
        self.first_obs_block == 0
    }

    fn has_max_observations(&self) -> bool {
        self.num_observations == MAX_OBSERVATIONS_PER_RECORD
    }

    fn has_half_max_observations(&self) -> bool {
        self.num_observations == MAX_OBSERVATIONS_PER_RECORD / 2
    }

    fn update(
        &mut self,
        current_block: u64,
        first_reserve: BigUint<M>,
        second_reserve: BigUint<M>,
    ) {
        if !self.is_default() {
            let current_weight = self.last_obs_block - self.first_obs_block + 1;
            let new_weight = current_block - self.last_obs_block;

            self.last_obs_block = current_block;
            self.num_observations += 1;
            self.first_token_reserve_weighted = (&self.first_token_reserve_weighted
                * current_weight
                + &self.first_token_reserve_last_obs * new_weight)
                / (current_weight + new_weight);
            self.second_token_reserve_weighted = (&self.second_token_reserve_weighted
                * current_weight
                + &self.second_token_reserve_last_obs * new_weight)
                / (current_weight + new_weight);

            self.first_token_reserve_last_obs = first_reserve;
            self.second_token_reserve_last_obs = second_reserve;
        }
    }
}

#[elrond_wasm::module]
pub trait SafePriceModule:
    config::ConfigModule + token_send::TokenSendModule + amm::AmmModule
{
    #[endpoint(updateAndGetTokensForGivenPositionWithSafePrice)]
    fn update_and_get_tokens_for_given_position_with_safe_price(
        &self,
        liquidity: BigUint,
    ) -> MultiResult2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> {
        self.update_safe_state_on_the_fly();

        let c_state = self.current_state().get();
        let total_supply = self.lp_token_supply().get();
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let big_zero = BigUint::zero();

        let (first_token_worth, second_token_worth) = if total_supply == big_zero
            || c_state.first_token_reserve_weighted == big_zero
            || c_state.second_token_reserve_weighted == big_zero
        {
            (big_zero.clone(), big_zero)
        } else {
            let first_worth = &liquidity * &c_state.first_token_reserve_weighted / &total_supply;
            let second_worth = &liquidity * &c_state.second_token_reserve_weighted / &total_supply;

            (first_worth, second_worth)
        };

        MultiResult2::from((
            EsdtTokenPayment::new(first_token_id, 0, first_token_worth),
            EsdtTokenPayment::new(second_token_id, 0, second_token_worth),
        ))
    }

    #[endpoint(updateAndGetSafePrice)]
    fn update_and_get_safe_price(
        &self,
        input: EsdtTokenPayment<Self::Api>,
    ) -> EsdtTokenPayment<Self::Api> {
        self.update_safe_state_on_the_fly();

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let c_state = self.current_state().get();

        let (r_in, r_out, t_out) = if input.token_identifier == first_token_id {
            let r_in = c_state.first_token_reserve_weighted.clone();
            let r_out = c_state.second_token_reserve_weighted;
            let t_out = second_token_id;

            (r_in, r_out, t_out)
        } else if input.token_identifier == second_token_id {
            let r_in = c_state.second_token_reserve_weighted.clone();
            let r_out = c_state.first_token_reserve_weighted;
            let t_out = first_token_id;

            (r_in, r_out, t_out)
        } else {
            sc_panic!(ERROR_UNKNOWN_TOKEN);
        };
        require!(
            input.amount != 0u64 && r_in != 0u64 && r_out != 0u64,
            ERROR_ZERO_AMOUNT
        );

        EsdtTokenPayment::new(t_out, 0, self.quote(&input.amount, &r_in, &r_out))
    }

    fn update_safe_state_from_context(&self, ctx: &dyn Context<Self::Api>) {
        self.update_safe_state(
            ctx.get_first_token_reserve(),
            ctx.get_second_token_reserve(),
        )
    }

    fn update_safe_state_on_the_fly(&self) {
        self.update_safe_state(
            &self.pair_reserve(&self.first_token_id().get()).get(),
            &self.pair_reserve(&self.second_token_id().get()).get(),
        );
    }

    fn update_safe_state(&self, first_token_reserve: &BigUint, second_token_reserve: &BigUint) {
        let current_block = self.blockchain().get_block_nonce();
        let mut current_state = self.get_current_state();
        let mut future_state = self.get_future_state();

        //Skip executing the update more than once per block.
        if current_state.contains_block(current_block) {
            return;
        }

        //Will be executed just once to initialize the current state.
        if current_state.is_default() {
            current_state =
                CumulativeState::new(current_block, first_token_reserve, second_token_reserve);
        }

        //Will be executed just once to initialize the future state.
        if current_state.has_half_max_observations() && future_state.is_default() {
            future_state =
                CumulativeState::new(current_block, first_token_reserve, second_token_reserve);
        }

        //At this point, future state is already initialized and contains half
        //of the observations that the current state contains.
        if current_state.has_max_observations() {
            current_state = future_state.clone();
            future_state =
                CumulativeState::new(current_block, first_token_reserve, second_token_reserve);
        }

        current_state.update(
            current_block,
            first_token_reserve.clone(),
            second_token_reserve.clone(),
        );
        future_state.update(
            current_block,
            first_token_reserve.clone(),
            second_token_reserve.clone(),
        );

        self.commit_states(current_state, future_state);
    }

    fn commit_states(
        &self,
        current: CumulativeState<Self::Api>,
        future: CumulativeState<Self::Api>,
    ) {
        if !current.is_default() {
            self.current_state().set(&current);
        }
        if !future.is_default() {
            self.future_state().set(&future);
        }
    }

    #[inline]
    fn get_current_state(&self) -> CumulativeState<Self::Api> {
        if self.current_state().is_empty() {
            Default::default()
        } else {
            self.current_state().get()
        }
    }

    #[inline]
    fn get_future_state(&self) -> CumulativeState<Self::Api> {
        if self.future_state().is_empty() {
            Default::default()
        } else {
            self.future_state().get()
        }
    }

    #[storage_mapper("current_state")]
    fn current_state(&self) -> SingleValueMapper<CumulativeState<Self::Api>>;

    #[storage_mapper("future_state")]
    fn future_state(&self) -> SingleValueMapper<CumulativeState<Self::Api>>;
}
