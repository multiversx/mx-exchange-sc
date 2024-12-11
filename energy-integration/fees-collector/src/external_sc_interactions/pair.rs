multiversx_sc::imports!();

mod pair_proxy {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait PairProxy {
        #[payable("*")]
        #[endpoint(swapTokensFixedInput)]
        fn swap_tokens_fixed_input(
            &self,
            token_out: TokenIdentifier,
            amount_out_min: BigUint, // TOOD: Must be at least 1
        ) -> EsdtTokenPayment;
    }
}

const TOKEN_OUT_MIN: u32 = 1;

#[multiversx_sc::module]
pub trait PairInteractionsModule {
    fn swap_to_common_token(
        &self,
        pair_address: ManagedAddress,
        input_payment: EsdtTokenPayment,
        token_out: TokenIdentifier,
    ) -> EsdtTokenPayment {
        self.pair_proxy_builder(pair_address)
            .swap_tokens_fixed_input(token_out, BigUint::from(TOKEN_OUT_MIN))
            .with_esdt_transfer(input_payment)
            .execute_on_dest_context()
    }

    #[proxy]
    fn pair_proxy_builder(&self, sc_address: ManagedAddress) -> pair_proxy::Proxy<Self::Api>;
}
