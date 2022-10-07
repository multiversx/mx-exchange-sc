use common_structs::Nonce;
use fixed_supply_token::FixedSupplyToken;

use crate::wrapped_lp_attributes::WrappedLpTokenAttributes;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub static INVALID_PAYMENTS_ERR_MSG: &[u8] = b"Invalid payments";
pub const MIN_MERGE_PAYMENTS: usize = 2;
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
        let second_is_locked = token_mapper.contains(&second_payment.token_identifier);

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
        let base_asset_token_id = self.asset_token().get_token_id();
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
            self.asset_token().get_token_id()
        } else {
            token_id
        }
    }

    fn get_underlying_locked_token(
        &self,
        token_id: TokenIdentifier,
        token_nonce: Nonce,
    ) -> TokenIdentifier {
        if self.locked_token_ids().contains(&token_id) {
            return token_id;
        }

        let wrapped_lp_token_mapper = self.wrapped_lp_token();
        wrapped_lp_token_mapper.require_same_token(&token_id);

        let attributes: WrappedLpTokenAttributes<Self::Api> =
            wrapped_lp_token_mapper.get_token_attributes(token_nonce);
        attributes.locked_tokens.token_identifier
    }

    fn burn_if_base_asset(&self, tokens: &EsdtTokenPayment) {
        let asset_token_id = self.asset_token().get_token_id();
        if tokens.token_identifier == asset_token_id {
            self.send()
                .esdt_local_burn(&tokens.token_identifier, 0, &tokens.amount);
        }
    }

    fn get_attributes_as_part_of_fixed_supply<T: FixedSupplyToken<Self::Api> + TopDecode>(
        &self,
        payment: &EsdtTokenPayment,
        mapper: &NonFungibleTokenMapper<Self::Api>,
    ) -> T {
        let attr: T = mapper.get_token_attributes(payment.token_nonce);
        attr.into_part(&payment.amount)
    }

    #[view(getAssetTokenId)]
    #[storage_mapper("assetTokenId")]
    fn asset_token(&self) -> FungibleTokenMapper<Self::Api>;

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
}
