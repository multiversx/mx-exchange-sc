elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait Lib {
    fn get_vote_weight(&self, _payment: &EsdtTokenPayment<Self::Api>) -> BigUint {
        todo!()
    }

    fn send_back(&self, _payment: EsdtTokenPayment<Self::Api>) {
        todo!()
    }
}
