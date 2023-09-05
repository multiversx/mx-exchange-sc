#![no_std]

multiversx_sc::imports!();

use common_structs::{PaymentAttributesPair, PaymentsVec};
use fixed_supply_token::FixedSupplyToken;
use mergeable::Mergeable;

static ERR_EMPTY_PAYMENTS: &[u8] = b"No payments";

#[multiversx_sc::module]
pub trait UtilsModule {
    fn dest_from_optional(&self, opt_destination: OptionalValue<ManagedAddress>) -> ManagedAddress {
        match opt_destination {
            OptionalValue::Some(dest) => dest,
            OptionalValue::None => self.blockchain().get_caller(),
        }
    }

    fn to_esdt_payment(
        &self,
        egld_or_esdt_payment: EgldOrEsdtTokenPayment<Self::Api>,
    ) -> EsdtTokenPayment {
        EsdtTokenPayment::new(
            egld_or_esdt_payment.token_identifier.unwrap_esdt(),
            egld_or_esdt_payment.token_nonce,
            egld_or_esdt_payment.amount,
        )
    }

    fn get_non_empty_payments(&self) -> PaymentsVec<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        require!(!payments.is_empty(), ERR_EMPTY_PAYMENTS);

        payments.clone_value()
    }

    fn pop_first_payment(
        &self,
        payments: &mut PaymentsVec<Self::Api>,
    ) -> EsdtTokenPayment<Self::Api> {
        require!(!payments.is_empty(), ERR_EMPTY_PAYMENTS);

        let first_payment = payments.get(0);
        payments.remove(0);

        first_payment
    }

    fn get_attributes_as_part_of_fixed_supply<T: FixedSupplyToken<Self::Api> + TopDecode>(
        &self,
        payment: &EsdtTokenPayment,
        mapper: &NonFungibleTokenMapper,
    ) -> T {
        let attr: T = mapper.get_token_attributes(payment.token_nonce);
        attr.into_part(&payment.amount)
    }

    fn merge_from_payments_and_burn<
        T: FixedSupplyToken<Self::Api> + Mergeable<Self::Api> + TopDecode,
    >(
        &self,
        mut payments: PaymentsVec<Self::Api>,
        mapper: &NonFungibleTokenMapper,
    ) -> T {
        let first_payment = self.pop_first_payment(&mut payments);
        let base_attributes: T =
            self.get_attributes_as_part_of_fixed_supply(&first_payment, mapper);
        mapper.nft_burn(first_payment.token_nonce, &first_payment.amount);

        let output_attributes =
            self.merge_attributes_from_payments(base_attributes, &payments, mapper);
        self.send().esdt_local_burn_multi(&payments);

        output_attributes
    }

    fn merge_attributes_from_payments<
        T: FixedSupplyToken<Self::Api> + Mergeable<Self::Api> + TopDecode,
    >(
        &self,
        mut base_attributes: T,
        payments: &PaymentsVec<Self::Api>,
        mapper: &NonFungibleTokenMapper,
    ) -> T {
        for payment in payments {
            let attributes: T = self.get_attributes_as_part_of_fixed_supply(&payment, mapper);
            base_attributes.merge_with(attributes);
        }

        base_attributes
    }

    fn merge_and_create_token<
        T: FixedSupplyToken<Self::Api>
            + Mergeable<Self::Api>
            + Clone
            + TopEncode
            + TopDecode
            + NestedEncode
            + NestedDecode,
    >(
        &self,
        base_attributes: T,
        payments: &PaymentsVec<Self::Api>,
        mapper: &NonFungibleTokenMapper,
    ) -> PaymentAttributesPair<Self::Api, T> {
        let output_attributes =
            self.merge_attributes_from_payments(base_attributes, payments, mapper);
        let new_token_amount = output_attributes.get_total_supply();
        let new_token_payment = mapper.nft_create(new_token_amount, &output_attributes);

        PaymentAttributesPair {
            payment: new_token_payment,
            attributes: output_attributes,
        }
    }

    fn require_valid_token_id(&self, token_id: &TokenIdentifier) {
        require!(token_id.is_valid_esdt_identifier(), "Invalid token ID");
    }

    fn require_sc_address(&self, address: &ManagedAddress) {
        require!(
            !address.is_zero() && self.blockchain().is_smart_contract(address),
            "Invalid SC address"
        );
    }
}
