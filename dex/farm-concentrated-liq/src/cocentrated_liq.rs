multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub type Tick = i32;

// TODO: Import from pair
pub const PRICE_DECIMALS: u64 = 1_000_000_000_000_000_000;
pub const PRICE_INCREASE_PER_TICK: i32 = 10_001;
pub const PRICE_SCALING_FACTOR: i32 = 10_000;

// TODO: Import from pair
#[derive(TypeAbi, NestedEncode, NestedDecode, TopEncode, TopDecode, Clone)]
pub struct LpTokenAttributes<M: ManagedTypeApi> {
    pub virtual_liquidity: BigUint<M>,
    pub tick_min: Tick,
    pub tick_max: Tick,
    pub first_token_accumulated_fee: BigUint<M>,
    pub second_token_accumulated_fee: BigUint<M>,
}

pub struct PriceBounds<M: ManagedTypeApi> {
    pub lower: BigUint<M>,
    pub upper: BigUint<M>,
}

#[multiversx_sc::module]
pub trait ConcentratedLiqModule:
    farm_token::FarmTokenModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn update_token_amounts_after_enter(
        &self,
        payment_amount: &BigUint,
        attributes: &LpTokenAttributes<Self::Api>,
    ) {
        let price_min_adjusted = self.tick_to_closest_min_ticker_multiply(attributes.tick_min);
        let price_max_adjusted = self.tick_to_closest_min_ticker_multiply(attributes.tick_max);

        self.tokens_with_min(&price_min_adjusted)
            .update(|value| *value += payment_amount);
        self.tokens_with_max(&price_max_adjusted)
            .update(|value| *value += payment_amount);
    }

    fn update_token_amounts_after_exit(
        &self,
        farming_token_amount: &BigUint,
        attributes: &LpTokenAttributes<Self::Api>,
    ) {
        let price_min_adjusted = self.tick_to_closest_min_ticker_multiply(attributes.tick_min);
        let price_max_adjusted = self.tick_to_closest_min_ticker_multiply(attributes.tick_max);

        self.tokens_with_min(&price_min_adjusted)
            .update(|value| *value -= farming_token_amount);
        self.tokens_with_max(&price_max_adjusted)
            .update(|value| *value -= farming_token_amount);
    }

    #[inline(always)]
    fn update_token_amounts_after_compound(
        &self,
        compounded_amount: &BigUint,
        attributes: &LpTokenAttributes<Self::Api>,
    ) {
        self.update_token_amounts_after_enter(compounded_amount, attributes);
    }

    // TODO: Query price!
    fn get_price_and_update_farm_token_supply(&self, _lp_token: EsdtTokenPayment) -> BigUint {
        let queried_price = BigUint::zero();

        let last_queried_price = self.last_queried_price().get();
        if last_queried_price == 0 {
            self.last_queried_price().set(&queried_price);

            return queried_price;
        }

        if queried_price == last_queried_price {
            return queried_price;
        }

        let price_bounds = self.get_price_bounds(&last_queried_price);
        let mut current_price = if queried_price > last_queried_price {
            price_bounds.upper
        } else {
            price_bounds.lower
        };

        let min_ticker = self.min_ticker().get();
        self.farm_token_supply().update(|farm_token_supply| {
            if queried_price > last_queried_price {
                while current_price <= queried_price {
                    let tokens_with_min = self.tokens_with_min(&current_price).get();
                    let tokens_with_max = self.tokens_with_max(&current_price).get();

                    *farm_token_supply += tokens_with_min;
                    *farm_token_supply -= tokens_with_max;

                    current_price += &min_ticker;
                }
            } else {
                while current_price >= queried_price {
                    let tokens_with_min = self.tokens_with_min(&current_price).get();
                    let tokens_with_max = self.tokens_with_max(&current_price).get();

                    *farm_token_supply -= tokens_with_min;
                    *farm_token_supply += tokens_with_max;

                    current_price -= &min_ticker;
                }
            }
        });

        self.last_queried_price().set(&queried_price);

        queried_price
    }

    fn tick_to_price(&self, tick: Tick) -> BigUint {
        let price_base = BigFloat::from(PRICE_INCREASE_PER_TICK) / PRICE_SCALING_FACTOR.into();
        let price = price_base.pow(tick);
        let price_scaled_down = price
            .ceil()
            .into_big_uint()
            .unwrap_or_else(|| sc_panic!("Could not convert to BigUint"));

        price_scaled_down * PRICE_DECIMALS
    }

    fn price_to_closest_min_ticker_multiply(&self, price: &BigUint) -> BigUint {
        let price_bounds = self.get_price_bounds(price);
        let lower_diff = price - &price_bounds.lower;
        let upper_diff = &price_bounds.upper - price;
        if lower_diff < upper_diff {
            price_bounds.lower
        } else {
            price_bounds.upper
        }
    }

    fn get_price_bounds(&self, price: &BigUint) -> PriceBounds<Self::Api> {
        let min_ticker = self.min_ticker().get();
        let lower_bound = price / &min_ticker * &min_ticker;
        let upper_bound = &lower_bound + &min_ticker;

        PriceBounds {
            lower: lower_bound,
            upper: upper_bound,
        }
    }

    fn tick_to_closest_min_ticker_multiply(&self, tick: Tick) -> BigUint {
        let price = self.tick_to_price(tick);
        self.price_to_closest_min_ticker_multiply(&price)
    }

    fn price_to_tick(&self, price: BigUint) -> Tick {
        let log_numerator = BigFloat::from(price) / BigFloat::from(BigUint::from(PRICE_DECIMALS));
        let log_base =
            BigFloat::from(PRICE_INCREASE_PER_TICK) / BigFloat::from(PRICE_SCALING_FACTOR);

        self.log_base_n(log_numerator, log_base)
    }

    // TODO: Find a better solution
    fn log_base_n(&self, numerator: BigFloat, base: BigFloat) -> i32 {
        let mut result = 0;
        let mut num = numerator;
        while num >= base {
            num /= &base;
            result += 1;
        }

        result
    }

    #[storage_mapper("minTicker")]
    fn min_ticker(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("tokensWithMin")]
    fn tokens_with_min(&self, price_min: &BigUint) -> SingleValueMapper<BigUint>;

    #[storage_mapper("tokensWithMax")]
    fn tokens_with_max(&self, price_max: &BigUint) -> SingleValueMapper<BigUint>;

    #[storage_mapper("lastQueriedPrice")]
    fn last_queried_price(&self) -> SingleValueMapper<BigUint>;
}
