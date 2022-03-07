elrond_wasm::imports!();

use crate::config;

mod price_provider_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait PriceProvider {
        #[endpoint(updateAndGetTokensForGivenPositionWithSafePrice)]
        fn update_and_get_tokens_for_given_position_with_safe_price(
            &self,
            liquidity: BigUint,
        ) -> MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>>;
    }
}

#[elrond_wasm::module]
pub trait Lib: config::Config {
    fn get_vote_weight(&self, payment: &EsdtTokenPayment<Self::Api>) -> BigUint {
        if payment.token_identifier == self.mex_token_id().get() {
            return payment.amount.clone();
        }

        if let Some(provider) = self.price_providers().get(&payment.token_identifier) {
            let (token1, token2) = self
                .price_provider_proxy(provider)
                .update_and_get_tokens_for_given_position_with_safe_price(payment.amount.clone())
                .execute_on_dest_context_custom_range(|_, after| (after - 2, after))
                .into_tuple();

            if payment.token_identifier == token1.token_identifier {
                return token1.amount;
            }

            if payment.token_identifier == token2.token_identifier {
                return token2.amount;
            }
        }

        BigUint::zero()
    }

    fn send_back(&self, payment: EsdtTokenPayment<Self::Api>) {
        self.send().direct(
            &self.blockchain().get_caller(),
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
            &[],
        );
    }

    #[proxy]
    fn price_provider_proxy(&self, to: ManagedAddress) -> price_provider_proxy::Proxy<Self::Api>;
}
