elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait UtilModule {
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

    fn merge_payments(
        &self,
        first_payment: &mut EsdtTokenPayment,
        second_payment: EsdtTokenPayment,
    ) {
        require!(
            first_payment.token_identifier == second_payment.token_identifier
                && first_payment.token_nonce == second_payment.token_nonce,
            "Cannot merge payments"
        );

        first_payment.amount += second_payment.amount;
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
