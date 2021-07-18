use crate::payment_receiver;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const OPERATION_GAS_LIMIT: u64 = 50_000_000;
use payment_receiver::ACCEPT_PAY_FUNC_NAME;

mod egld_wrap_mod {
    elrond_wasm::imports!();

    #[elrond_wasm_derive::proxy]
    pub trait EgldWrapContract {
        #[payable("EGLD")]
        #[endpoint(wrapEgld)]
        fn wrap_egld(
            &self,
            #[payment] payment: Self::BigUint,
            #[var_args] accept_funds_endpoint_name: OptionalArg<BoxedBytes>,
        ) -> OptionalResult<AsyncCall<Self::SendApi>>;

        #[payable("*")]
        #[endpoint(unwrapEgld)]
        fn unwrap_egld(
            &self,
            #[payment] payment: Self::BigUint,
            #[payment_token] token_id: TokenIdentifier,
            #[var_args] accept_funds_endpoint_name: OptionalArg<BoxedBytes>,
        ) -> OptionalResult<AsyncCall<Self::SendApi>>;
    }
}

#[elrond_wasm_derive::module]
pub trait EgldWrapProxyModule: payment_receiver::PaymentReceivedModule {
    #[proxy]
    fn egld_wrap_proxy(&self, to: Address) -> egld_wrap_mod::Proxy<Self::SendApi>;

    fn wrap_egld(&self, amount: &Self::BigUint) -> SCResult<()> {
        let own_address = self.blockchain().get_sc_address();
        let balance_before = self.blockchain().get_balance(&own_address);

        let _ = self
            .egld_wrap_proxy(self.egld_wrap_contract_address().get())
            .wrap_egld(
                amount.clone(),
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .with_token_transfer(TokenIdentifier::egld(), amount.clone())
            .with_gas_limit(OPERATION_GAS_LIMIT)
            .execute_on_dest_context();

        let balance_after = self.blockchain().get_balance(&own_address);
        require!(
            balance_after < balance_before && &(balance_before - balance_after) == amount,
            "Wrapping failed"
        );
        Ok(())
    }

    fn unwrap_egld(&self, amount: &Self::BigUint) -> SCResult<()> {
        let wegld = self.wegld_token_id().get();
        let own_address = self.blockchain().get_sc_address();
        let balance_before = self.blockchain().get_balance(&own_address);

        let _ = self
            .egld_wrap_proxy(self.egld_wrap_contract_address().get())
            .unwrap_egld(
                amount.clone(),
                wegld.clone(),
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .with_token_transfer(wegld, amount.clone())
            .with_gas_limit(OPERATION_GAS_LIMIT)
            .execute_on_dest_context_ignore_result();

        let balance_after = self.blockchain().get_balance(&own_address);

        require!(
            balance_after > balance_before && &(balance_after - balance_before) == amount,
            "Unwrapping failed"
        );
        Ok(())
    }

    #[view(getWegldTokenId)]
    #[storage_mapper("wegld_token_id")]
    fn wegld_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getEgldWrapContractAddress)]
    #[storage_mapper("egld_wrap_contract_address")]
    fn egld_wrap_contract_address(&self) -> SingleValueMapper<Self::Storage, Address>;
}
