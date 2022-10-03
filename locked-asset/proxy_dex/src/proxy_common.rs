elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Nonce;

use crate::{
    wrapped_farm_attributes::WrappedFarmTokenAttributes,
    wrapped_lp_attributes::WrappedLpTokenAttributes,
};

/// common interface for both old and new locked token factory
pub mod locked_token_factory {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait LockedTokenFactory {
        #[payable("*")]
        #[endpoint(mergeTokens)]
        fn merge_tokens(&self) -> EsdtTokenPayment;
    }
}

#[elrond_wasm::module]
pub trait ProxyCommonModule: token_send::TokenSendModule {
    fn get_wrapped_lp_token_attributes(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
    ) -> WrappedLpTokenAttributes<Self::Api> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        token_info.decode_attributes()
    }

    fn get_wrapped_farm_token_attributes(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
    ) -> WrappedFarmTokenAttributes<Self::Api> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        token_info.decode_attributes()
    }

    fn burn_payment_tokens(
        &self,
        payments: ManagedVecRefIterator<Self::Api, EsdtTokenPayment<Self::Api>>,
    ) {
        for payment in payments {
            self.send().esdt_local_burn(
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );
        }
    }

    #[view(getAssetTokenId)]
    #[storage_mapper("assetTokenId")]
    fn asset_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLockedTokenIds)]
    #[storage_mapper("lockedTokenIds")]
    fn locked_token_ids(&self) -> UnorderedSetMapper<TokenIdentifier>;

    #[storage_mapper("factoryAddressForLockedToken")]
    fn factory_address_for_locked_token(
        &self,
        locked_token_id: &TokenIdentifier,
    ) -> SingleValueMapper<ManagedAddress>;

    #[view(getWrappedLpTokenId)]
    #[storage_mapper("wrappedLpTokenId")]
    fn wrapped_lp_token(&self) -> NonFungibleTokenMapper<Self::Api>;

    #[view(getWrappedFarmTokenId)]
    #[storage_mapper("wrappedFarmTokenId")]
    fn wrapped_farm_token(&self) -> NonFungibleTokenMapper<Self::Api>;

    #[view(getIntermediatedFarms)]
    #[storage_mapper("intermediatedFarms")]
    fn intermediated_farms(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[view(getIntermediatedPairs)]
    #[storage_mapper("intermediatedPairs")]
    fn intermediated_pairs(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[proxy]
    fn farm_contract_proxy(&self, to: ManagedAddress) -> farm::Proxy<Self::Api>;

    #[proxy]
    fn locked_token_factory_proxy(
        &self,
        to: ManagedAddress,
    ) -> locked_token_factory::Proxy<Self::Api>;
}
