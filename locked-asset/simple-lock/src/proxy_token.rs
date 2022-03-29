elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub enum ProxyTokenAttributes<M: ManagedTypeApi> {
    LpProxyToken {
        attributes: LpProxyTokenAttributes<M>,
    },
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct LpProxyTokenAttributes<M: ManagedTypeApi> {
    pub lp_address: ManagedAddress<M>,
    pub lp_token_id: TokenIdentifier<M>,
    pub first_token_id: TokenIdentifier<M>,
    pub first_token_locked_nonce: u64,
    pub second_token_id: TokenIdentifier<M>,
    pub second_token_locked_nonce: u64,
}

#[elrond_wasm::module]
pub trait ProxyTokenModule:
    elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueProxyToken)]
    fn issue_proxy_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        let payment_amount = self.call_value().egld_value();

        self.proxy_token().issue(
            EsdtTokenType::Meta,
            payment_amount,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    #[only_owner]
    #[endpoint(setLocalRolesProxyToken)]
    fn set_local_roles_proxy_token(&self) {
        self.proxy_token().set_local_roles(
            &[
                EsdtLocalRole::NftCreate,
                EsdtLocalRole::NftAddQuantity,
                EsdtLocalRole::NftBurn,
            ],
            None,
        );
    }

    #[view(getProxyTokenId)]
    #[storage_mapper("proxyTokenId")]
    fn proxy_token(&self) -> NonFungibleTokenMapper<Self::Api>;
}
