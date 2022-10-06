elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Nonce;

use crate::{
    wrapped_farm_attributes::WrappedFarmTokenAttributes,
    wrapped_lp_attributes::WrappedLpTokenAttributes,
};

pub static FACTORY_MERGE_TOKENS_ENDPOINT_NAME: &[u8] = b"mergeTokens";
pub static INVALID_PAYMENTS_ERR_MSG: &[u8] = b"Invalid payments";

pub struct LockedUnlockedTokenRefPair<'a, M: ManagedTypeApi> {
    pub locked_token_ref: &'a EsdtTokenPayment<M>,
    pub unlocked_token_ref: &'a EsdtTokenPayment<M>,
}

pub struct BaseAssetOtherTokenRefPair<'a, M: ManagedTypeApi> {
    pub base_asset_token_ref: &'a EsdtTokenPayment<M>,
    pub other_token_ref: &'a EsdtTokenPayment<M>,
}

#[elrond_wasm::module]
pub trait ProxyCommonModule {
    fn require_exactly_one_locked<'a>(
        &self,
        first_payment: &'a EsdtTokenPayment,
        second_payment: &'a EsdtTokenPayment,
    ) -> LockedUnlockedTokenRefPair<'a, Self::Api> {
        let token_mapper = self.locked_token_ids();
        let first_is_locked = token_mapper.contains(&first_payment.token_identifier);
        let second_is_locked = token_mapper.contains(&first_payment.token_identifier);

        if first_is_locked {
            require!(!second_is_locked, INVALID_PAYMENTS_ERR_MSG);

            LockedUnlockedTokenRefPair {
                locked_token_ref: first_payment,
                unlocked_token_ref: second_payment,
            }
        } else {
            require!(second_is_locked, INVALID_PAYMENTS_ERR_MSG);

            LockedUnlockedTokenRefPair {
                locked_token_ref: second_payment,
                unlocked_token_ref: first_payment,
            }
        }
    }

    fn require_exactly_one_base_asset<'a>(
        &self,
        first_payment: &'a EsdtTokenPayment,
        second_payment: &'a EsdtTokenPayment,
    ) -> BaseAssetOtherTokenRefPair<'a, Self::Api> {
        let base_asset_token_id = self.asset_token_id().get();
        let is_first_token = first_payment.token_identifier == base_asset_token_id;
        let is_second_token = second_payment.token_identifier == base_asset_token_id;

        if is_first_token {
            require!(!is_second_token, INVALID_PAYMENTS_ERR_MSG);

            BaseAssetOtherTokenRefPair {
                base_asset_token_ref: first_payment,
                other_token_ref: second_payment,
            }
        } else {
            require!(is_second_token, INVALID_PAYMENTS_ERR_MSG);

            BaseAssetOtherTokenRefPair {
                base_asset_token_ref: second_payment,
                other_token_ref: first_payment,
            }
        }
    }

    fn get_underlying_token(&self, token_id: TokenIdentifier) -> TokenIdentifier {
        if self.locked_token_ids().contains(&token_id) {
            self.asset_token_id().get()
        } else {
            token_id
        }
    }

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

    #[proxy]
    fn farm_contract_proxy(&self, to: ManagedAddress) -> farm::Proxy<Self::Api>;
}
