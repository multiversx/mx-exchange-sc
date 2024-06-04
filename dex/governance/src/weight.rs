multiversx_sc::imports!();

use crate::config;
use crate::price_provider_proxy;

#[multiversx_sc::module]
pub trait Lib: config::Config {
    fn get_vote_weight(&self, payment: &EsdtTokenPayment<Self::Api>) -> BigUint {
        let mex_token_id = self.mex_token_id().get();

        if payment.token_identifier == self.mex_token_id().get() {
            return payment.amount.clone();
        }

        if let Some(provider) = self.price_providers().get(&payment.token_identifier) {
            let call_result = self
                .tx()
                .to(&provider)
                .typed(price_provider_proxy::PriceProviderProxy)
                .get_tokens_for_given_position_with_safe_price(payment.amount.clone())
                .returns(ReturnsResult)
                .sync_call();

            let (token1, token2) = call_result.into_tuple();

            if token1.token_identifier == mex_token_id {
                return token1.amount;
            }

            if token2.token_identifier == mex_token_id {
                return token2.amount;
            }
        }

        BigUint::zero()
    }

    fn send_back(&self, payment: EsdtTokenPayment<Self::Api>) {
        self.tx()
            .to(&self.blockchain().get_caller())
            .payment(payment)
            .transfer();
    }
}
