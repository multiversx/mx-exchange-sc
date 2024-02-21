multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub type Tick = i32;

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
        let price_min = self.tick_to_price(attributes.tick_min);
        let price_max = self.tick_to_price(attributes.tick_max);
        let price_min_adjusted = self.price_to_closest_min_ticker_multiply(&price_min);
        let price_max_adjusted = self.price_to_closest_min_ticker_multiply(&price_max);

        self.tokens_with_min(&price_min_adjusted)
            .update(|value| *value += payment_amount);
        self.tokens_with_max(&price_max_adjusted)
            .update(|value| *value += payment_amount);
    }

    fn get_price(&self, _lp_token: EsdtTokenPayment) -> BigUint {
        let last_queried_price = self.last_queried_price().get();
        if last_queried_price == 0 {
            todo!();
        }

        todo!()
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
        let min_ticker = self.min_ticker().get();
        let lower_bound = price / &min_ticker * &min_ticker; 
        let upper_bound = &lower_bound + &min_ticker;

        let lower_diff = price - &lower_bound;
        let upper_diff = &upper_bound - price;
        if lower_diff < upper_diff {
            lower_bound
        } else {
            upper_bound
        }
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
