use common_structs::Nonce;

use crate::wrapped_lp_attributes::WrappedLpTokenAttributes;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

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

#[multiversx_sc::module]
pub trait ProxyCommonModule: energy_query::EnergyQueryModule {
    fn require_exactly_one_locked<'a>(
        &self,
        first_payment: &'a EsdtTokenPayment,
        second_payment: &'a EsdtTokenPayment,
    ) -> LockedUnlockedTokenRefPair<'a, Self::Api> {
        let first_is_locked = self.is_locked_token(&first_payment.token_identifier);
        let second_is_locked = self.is_locked_token(&second_payment.token_identifier);

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
        let base_asset_token_id = self.get_base_token_id();
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
        if self.is_locked_token(&token_id) {
            self.get_base_token_id()
        } else {
            token_id
        }
    }

    fn get_underlying_locked_token(
        &self,
        token_id: TokenIdentifier,
        token_nonce: Nonce,
    ) -> TokenIdentifier {
        if self.is_locked_token(&token_id) {
            return token_id;
        }

        let wrapped_lp_token_mapper = self.wrapped_lp_token();
        wrapped_lp_token_mapper.require_same_token(&token_id);

        let attributes: WrappedLpTokenAttributes<Self::Api> =
            wrapped_lp_token_mapper.get_token_attributes(token_nonce);
        attributes.locked_tokens.token_identifier
    }

    fn is_locked_token(&self, token_id: &TokenIdentifier) -> bool {
        let new_locked_token_id = self.get_locked_token_id();
        if token_id == &new_locked_token_id {
            return true;
        }

        let old_locked_token_id = self.old_locked_token_id().get();
        token_id == &old_locked_token_id
    }

    fn get_factory_address_for_locked_token(&self, token_id: &TokenIdentifier) -> ManagedAddress {
        let new_locked_token_id = self.get_locked_token_id();
        if token_id == &new_locked_token_id {
            return self.energy_factory_address().get();
        }

        let old_locked_token_id = self.old_locked_token_id().get();
        require!(token_id == &old_locked_token_id, "Invalid locked token ID");

        self.old_factory_address().get()
    }

    fn burn_if_base_asset(&self, tokens: &EsdtTokenPayment) {
        let asset_token_id = self.get_base_token_id();
        if tokens.token_identifier == asset_token_id {
            self.send()
                .esdt_local_burn(&tokens.token_identifier, 0, &tokens.amount);
        }
    }

    #[view(getAssetTokenId)]
    fn get_asset_token_id_view(&self) -> TokenIdentifier {
        self.get_base_token_id()
    }

    #[view(getLockedTokenIds)]
    fn get_locked_token_ids_view(&self) -> MultiValueEncoded<TokenIdentifier> {
        let new_token_id = self.get_locked_token_id();
        let old_token_id = self.old_locked_token_id().get();
        let mut results = MultiValueEncoded::new();
        results.push(new_token_id);
        results.push(old_token_id);

        results
    }

    #[view(getOldLockedTokenId)]
    #[storage_mapper("oldLockedTokenId")]
    fn old_locked_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getOldFactoryAddress)]
    #[storage_mapper("oldFactoryAddress")]
    fn old_factory_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getWrappedLpTokenId)]
    #[storage_mapper("wrappedLpTokenId")]
    fn wrapped_lp_token(&self) -> NonFungibleTokenMapper;

    #[view(getWrappedFarmTokenId)]
    #[storage_mapper("wrappedFarmTokenId")]
    fn wrapped_farm_token(&self) -> NonFungibleTokenMapper;
}
