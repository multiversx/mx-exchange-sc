elrond_wasm::imports!();

use crate::config;
use crate::errors::*;

#[elrond_wasm::module]
pub trait Validation: config::Config {
    fn require_is_accepted_payment(&self, payment: &EsdtTokenPayment<Self::Api>) {
        require!(
            self.governance_token_ids()
                .contains(&payment.token_identifier),
            UNREGISTERED_TOKEN_ID
        );
    }
}
