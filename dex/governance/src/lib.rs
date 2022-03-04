elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait Lib {
    fn get_vote_weight(&self, payment: &EsdtTokenPayment<Self::Api>) -> BigUint {
        payment.amount.clone()
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
}
