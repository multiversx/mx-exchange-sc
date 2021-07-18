elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub const ACCEPT_PAY_FUNC_NAME: &[u8] = b"acceptPay";

#[elrond_wasm_derive::module]
pub trait PaymentReceivedModule {
    #[payable("*")]
    #[endpoint(acceptPay)]
    fn accept_pay(&self) {}
}
