elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Nonce;
use common_structs::{WrappedFarmTokenAttributes, WrappedLpTokenAttributes};

pub const ACCEPT_PAY_FUNC_NAME: &[u8] = b"acceptPay";

#[elrond_wasm_derive::module]
pub trait ProxyCommonModule {
    fn require_permissions(&self) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        Ok(())
    }

    #[payable("*")]
    #[endpoint(acceptPay)]
    fn accept_pay(&self) {}

    fn direct_generic_safe(
        &self,
        to: &Address,
        token_id: &TokenIdentifier,
        nonce: Nonce,
        amount: &Self::BigUint,
    ) {
        if amount > &0 {
            self.send().direct(to, token_id, nonce, amount, &[]);
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
    ) -> SCResult<WrappedLpTokenAttributes<Self::BigUint>> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        let attributes = token_info.decode_attributes::<WrappedLpTokenAttributes<Self::BigUint>>();
        match attributes {
            Result::Ok(decoded_obj) => Ok(decoded_obj),
            Result::Err(_) => {
                return sc_error!("Decoding error");
            }
        }
    }

    fn get_wrapped_farm_token_attributes(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
    ) -> SCResult<WrappedFarmTokenAttributes<Self::BigUint>> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        let attributes =
            token_info.decode_attributes::<WrappedFarmTokenAttributes<Self::BigUint>>();
        match attributes {
            Result::Ok(decoded_obj) => Ok(decoded_obj),
            Result::Err(_) => {
                return sc_error!("Decoding error");
            }
        }
    }

    #[storage_mapper("current_tx_accepted_funds")]
    fn current_tx_accepted_funds(
        &self,
    ) -> SafeMapMapper<Self::Storage, (TokenIdentifier, Nonce), Self::BigUint>;

    #[view(getAssetTokenId)]
    #[storage_mapper("asset_token_id")]
    fn asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getLockedAssetTokenId)]
    #[storage_mapper("locked_asset_token_id")]
    fn locked_asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getWrappedLpTokenId)]
    #[storage_mapper("wrapped_lp_token_id")]
    fn wrapped_lp_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[storage_mapper("wrapped_lp_token_nonce")]
    fn wrapped_lp_token_nonce(&self) -> SingleValueMapper<Self::Storage, Nonce>;

    #[view(getWrappedFarmTokenId)]
    #[storage_mapper("wrapped_farm_token_id")]
    fn wrapped_farm_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[storage_mapper("wrapped_farm_token_nonce")]
    fn wrapped_farm_token_nonce(&self) -> SingleValueMapper<Self::Storage, Nonce>;

    #[storage_mapper("locked_asset_factory_address")]
    fn locked_asset_factory_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getIntermediatedFarms)]
    #[storage_mapper("intermediated_farms")]
    fn intermediated_farms(&self) -> SafeSetMapper<Self::Storage, Address>;

    #[view(getIntermediatedPairs)]
    #[storage_mapper("intermediated_pairs")]
    fn intermediated_pairs(&self) -> SafeSetMapper<Self::Storage, Address>;
}
