multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Nonce;

pub static WRAPPED_TOKEN_NAME: &[u8] = b"WrappedLKMEX";

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct WrappedTokenAttributes {
    pub locked_token_nonce: Nonce,
}

#[multiversx_sc::module]
pub trait WrappedTokenModule:
    multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::token_attributes::TokenAttributesModule
{
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueWrappedToken)]
    fn issue_wrapped_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        let payment_amount = self.call_value().egld_value().clone_value();

        self.wrapped_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            payment_amount,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    /// Sets the transfer role for the given address. Defaults to own address.
    #[only_owner]
    #[endpoint(setTransferRoleWrappedToken)]
    fn set_transfer_role(&self, opt_address: OptionalValue<ManagedAddress>) {
        let address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };

        self.wrapped_token().set_local_roles_for_address(
            &address,
            &[EsdtLocalRole::Transfer],
            None,
        );
    }

    /// Removes the transfer role for the given address.
    #[only_owner]
    #[endpoint(unsetTransferRoleWrappedToken)]
    fn unset_transfer_role(&self, address: ManagedAddress) {
        let wrapped_token_id = self.wrapped_token().get_token_id();
        let system_sc_proxy = ESDTSystemSmartContractProxy::new_proxy_obj();
        system_sc_proxy
            .unset_special_roles(
                &address,
                &wrapped_token_id,
                [EsdtLocalRole::Transfer][..].iter().cloned(),
            )
            .async_call()
            .call_and_exit();
    }

    fn wrap_locked_token_and_send(
        &self,
        caller: &ManagedAddress,
        token: EsdtTokenPayment,
    ) -> EsdtTokenPayment {
        let wrapped_token_mapper = self.wrapped_token();
        let wrapped_token_attributes = WrappedTokenAttributes {
            locked_token_nonce: token.token_nonce,
        };
        let wrapped_token_nonce = self.get_or_create_nonce_for_attributes(
            &wrapped_token_mapper,
            &ManagedBuffer::new_from_bytes(WRAPPED_TOKEN_NAME),
            &wrapped_token_attributes,
        );

        wrapped_token_mapper.nft_add_quantity_and_send(caller, wrapped_token_nonce, token.amount)
    }

    fn unwrap_locked_token(
        &self,
        locked_token_id: TokenIdentifier,
        token: EsdtTokenPayment,
    ) -> EsdtTokenPayment {
        let wrapped_token_mapper = self.wrapped_token();
        wrapped_token_mapper.require_same_token(&token.token_identifier);

        let wrapped_token_attributes: WrappedTokenAttributes =
            wrapped_token_mapper.get_token_attributes(token.token_nonce);

        self.send()
            .esdt_local_burn(&token.token_identifier, token.token_nonce, &token.amount);

        EsdtTokenPayment::new(
            locked_token_id,
            wrapped_token_attributes.locked_token_nonce,
            token.amount,
        )
    }

    #[view(getWrappedTokenId)]
    #[storage_mapper("wrappedTokenId")]
    fn wrapped_token(&self) -> NonFungibleTokenMapper;
}
