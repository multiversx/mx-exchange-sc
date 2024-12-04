#![no_std]

multiversx_sc::imports!();

use common_structs::{FarmToken, PaymentsVec};

#[multiversx_sc::module]
pub trait OriginalOwnerHelperModule {
    fn check_and_return_original_owner<T: FarmToken<Self::Api> + TopDecode>(
        &self,
        payments: &PaymentsVec<Self::Api>,
        farm_token_mapper: &NonFungibleTokenMapper,
    ) -> ManagedAddress {
        let mut original_owner = ManagedAddress::zero();
        for payment in payments.iter() {
            let attributes: T = farm_token_mapper.get_token_attributes(payment.token_nonce);
            let payment_original_owner = attributes.get_original_owner();

            if original_owner.is_zero() {
                original_owner = payment_original_owner;
            } else {
                require!(
                    original_owner == payment_original_owner,
                    "All position must have the same original owner"
                );
            }
        }

        require!(
            !original_owner.is_zero(),
            "Original owner could not be identified"
        );

        original_owner
    }

    fn check_additional_payments_original_owner<T: FarmToken<Self::Api> + TopDecode>(
        &self,
        user: &ManagedAddress,
        payments: &PaymentsVec<Self::Api>,
        farm_token_mapper: &NonFungibleTokenMapper,
    ) {
        if payments.len() == 1 {
            return;
        }

        let farm_token_id = farm_token_mapper.get_token_id();
        for payment in payments.into_iter() {
            if payment.token_identifier != farm_token_id {
                continue;
            }

            let attributes: T = farm_token_mapper.get_token_attributes(payment.token_nonce);
            let payment_original_owner = attributes.get_original_owner();

            require!(
                user == &payment_original_owner,
                "Provided address is not the same as the original owner"
            );
        }
    }
}