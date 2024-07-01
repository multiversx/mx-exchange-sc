multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Nonce;
use common_structs::{WrappedFarmTokenAttributes, WrappedLpTokenAttributes};

#[multiversx_sc::module]
pub trait ProxyCommonModule {
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

    fn send_multiple_tokens_if_not_zero(
        &self,
        destination: &ManagedAddress,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) {
        let mut non_zero_payments = ManagedVec::new();
        for payment in payments {
            if payment.amount > 0u32 {
                non_zero_payments.push(payment);
            }
        }

        if !non_zero_payments.is_empty() {
            self.send().direct_multi(destination, &non_zero_payments)
        }
    }

    #[storage_mapper("current_tx_accepted_funds")]
    fn current_tx_accepted_funds(&self) -> MapMapper<(TokenIdentifier, Nonce), BigUint>;

    #[view(getAssetTokenId)]
    #[storage_mapper("asset_token_id")]
    fn asset_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLockedAssetTokenId)]
    #[storage_mapper("locked_asset_token_id")]
    fn locked_asset_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getWrappedLpTokenId)]
    #[storage_mapper("wrapped_lp_token_id")]
    fn wrapped_lp_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getWrappedFarmTokenId)]
    #[storage_mapper("wrapped_farm_token_id")]
    fn wrapped_farm_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("locked_asset_factory_address")]
    fn locked_asset_factory_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("intermediated_farms")]
    fn intermediated_farms(&self) -> SetMapper<ManagedAddress>;

    #[view(getIntermediatedFarms)]
    fn get_intermediated_farms(&self) -> MultiValueEncoded<ManagedAddress> {
        let mut result = MultiValueEncoded::new();
        for pair in self.intermediated_farms().iter() {
            result.push(pair);
        }
        result
    }

    #[storage_mapper("intermediated_pairs")]
    fn intermediated_pairs(&self) -> SetMapper<ManagedAddress>;

    #[view(getIntermediatedPairs)]
    fn get_intermediated_pairs(&self) -> MultiValueEncoded<ManagedAddress> {
        let mut result = MultiValueEncoded::new();
        for pair in self.intermediated_pairs().iter() {
            result.push(pair);
        }
        result
    }
}
