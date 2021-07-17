elrond_wasm::imports!();
elrond_wasm::derive_imports!();

mod egld_wrap_mod {
    elrond_wasm::imports!();

    #[elrond_wasm_derive::proxy]
    pub trait EgldWrapContract {
        #[payable("EGLD")]
        #[endpoint(wrapEgld)]
        fn wrap_egld(&self, #[payment] payment: Self::BigUint);

        #[payable("*")]
        #[endpoint(unwrapEgld)]
        fn unwrap_egld(
            &self,
            #[payment] payment: Self::BigUint,
            #[payment_token] token_id: TokenIdentifier,
        );
    }
}

#[elrond_wasm_derive::module]
pub trait EgldWrapProxyModule {
    #[proxy]
    fn egld_wrap_proxy(&self, to: Address) -> egld_wrap_mod::Proxy<Self::SendApi>;

    fn wrap_egld(&self, amount: &Self::BigUint) {
        self.egld_wrap_proxy(self.egld_wrap_contract_address().get())
            .wrap_egld(amount.clone())
            .with_token_transfer(TokenIdentifier::egld(), amount.clone())
            .execute_on_dest_context_ignore_result();
    }

    fn unwrap_egld(&self, amount: &Self::BigUint) {
        let wegld = self.wegld_token_id().get();

        self.egld_wrap_proxy(self.egld_wrap_contract_address().get())
            .unwrap_egld(amount.clone(), wegld.clone())
            .with_token_transfer(wegld, amount.clone())
            .execute_on_dest_context_ignore_result();
    }

    #[view(getWegldTokenId)]
    #[storage_mapper("wegld_token_id")]
    fn wegld_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getEgldWrapContractAddress)]
    #[storage_mapper("egld_wrap_contract_address")]
    fn egld_wrap_contract_address(&self) -> SingleValueMapper<Self::Storage, Address>;
}
