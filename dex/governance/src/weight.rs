multiversx_sc::imports!();

use crate::config;

mod price_provider_proxy {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait PriceProvider {
        #[endpoint(getTokensForGivenPositionWithSafePrice)]
        fn get_tokens_for_given_position_with_safe_price(
            &self,
            liquidity: BigUint,
        ) -> MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>>;
    }
}

#[multiversx_sc::module]
pub trait Lib: config::Config {
    fn get_vote_weight(&self, payment: &EsdtTokenPayment<Self::Api>) -> BigUint {
        let mex_token_id = self.mex_token_id().get();

        if payment.token_identifier == self.mex_token_id().get() {
            return payment.amount.clone();
        }

        if let Some(provider) = self.price_providers().get(&payment.token_identifier) {
            let call_result: MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> =
                self.price_provider_proxy(provider)
                    .get_tokens_for_given_position_with_safe_price(payment.amount.clone())
                    .execute_on_dest_context();
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
        self.send().direct_esdt(
            &self.blockchain().get_caller(),
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );
    }

    #[proxy]
    fn price_provider_proxy(&self, to: ManagedAddress) -> price_provider_proxy::Proxy<Self::Api>;
}
