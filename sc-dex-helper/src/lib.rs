#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod elgd_wrap_proxy;
mod pair_helper;
mod payment_receiver;

#[elrond_wasm_derive::contract]
pub trait DexHelper:
    pair_helper::PairHelperModule
    + elgd_wrap_proxy::EgldWrapProxyModule
    + payment_receiver::PaymentReceivedModule
{
    #[init]
    fn init(&self, wegld_token_id: TokenIdentifier, egld_wrap_contract_address: Address) {
        self.wegld_token_id().set(&wegld_token_id);
        self.egld_wrap_contract_address()
            .set(&egld_wrap_contract_address);
    }
}
