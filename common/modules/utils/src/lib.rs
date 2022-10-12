#![no_std]

elrond_wasm::imports!();

use common_structs::PaymentsVec;
use fixed_supply_token::FixedSupplyToken;

static ERR_EMPTY_PAYMENTS: &[u8] = b"No payments";

#[elrond_wasm::module]
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

    fn burn_multi_esdt(&self, payments: &PaymentsVec<Self::Api>) {
        for payment in payments {
            self.send().esdt_local_burn(
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );
        }
    }

    fn get_non_empty_payments(&self) -> PaymentsVec<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        require!(!payments.is_empty(), ERR_EMPTY_PAYMENTS);

        payments
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
        mapper: &NonFungibleTokenMapper<Self::Api>,
    ) -> T {
        let attr: T = mapper.get_token_attributes(payment.token_nonce);
        attr.into_part(&payment.amount)
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
