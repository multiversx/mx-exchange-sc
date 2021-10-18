elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Nonce;
use common_structs::{WrappedFarmTokenAttributes, WrappedLpTokenAttributes};

pub const ACCEPT_PAY_FUNC_NAME: &[u8] = b"acceptPay";

#[elrond_wasm::module]
pub trait ProxyCommonModule: token_send::TokenSendModule {
    fn require_permissions(&self) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        Ok(())
    }

    #[payable("*")]
    #[endpoint(acceptPay)]
    fn accept_pay(&self) {}

    fn direct_generic_safe(
        &self,
        to: &ManagedAddress,
        token_id: &TokenIdentifier,
        nonce: Nonce,
        amount: &BigUint,
    ) -> SCResult<()> {
        if amount > &0 {
            self.direct_esdt_nft_execute_custom(to, token_id, nonce, amount, &OptionalArg::None)
        } else {
            Ok(())
        }
    }

    fn increase_wrapped_lp_token_nonce(&self) -> Nonce {
        let new_nonce = self.wrapped_lp_token_nonce().get() + 1;
        self.wrapped_lp_token_nonce().set(&new_nonce);
        new_nonce
    }

    fn increase_wrapped_farm_token_nonce(&self) -> Nonce {
        let new_nonce = self.wrapped_farm_token_nonce().get() + 1;
        self.wrapped_farm_token_nonce().set(&new_nonce);
        new_nonce
    }

    fn get_wrapped_lp_token_attributes(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
    ) -> SCResult<WrappedLpTokenAttributes<Self::Api>> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        Ok(self
            .serializer()
            .top_decode_from_managed_buffer::<WrappedLpTokenAttributes<Self::Api>>(
                &token_info.attributes,
            ))
    }

    fn get_wrapped_farm_token_attributes(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
    ) -> SCResult<WrappedFarmTokenAttributes<Self::Api>> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        Ok(self
            .serializer()
            .top_decode_from_managed_buffer::<WrappedFarmTokenAttributes<Self::Api>>(
                &token_info.attributes,
            ))
    }

    fn burn_payment_tokens(&self, payments: &[EsdtTokenPayment<Self::Api>]) {
        for payment in payments.iter() {
            self.send().esdt_local_burn(
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );
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

    #[storage_mapper("wrapped_lp_token_nonce")]
    fn wrapped_lp_token_nonce(&self) -> SingleValueMapper<Nonce>;

    #[view(getWrappedFarmTokenId)]
    #[storage_mapper("wrapped_farm_token_id")]
    fn wrapped_farm_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("wrapped_farm_token_nonce")]
    fn wrapped_farm_token_nonce(&self) -> SingleValueMapper<Nonce>;

    #[storage_mapper("locked_asset_factory_address")]
    fn locked_asset_factory_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getIntermediatedFarms)]
    #[storage_mapper("intermediated_farms")]
    fn intermediated_farms(&self) -> SetMapper<ManagedAddress>;

    #[view(getIntermediatedPairs)]
    #[storage_mapper("intermediated_pairs")]
    fn intermediated_pairs(&self) -> SetMapper<ManagedAddress>;
}
