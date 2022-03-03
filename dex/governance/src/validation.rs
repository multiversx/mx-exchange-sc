elrond_wasm::imports!();

use crate::config;
use crate::errors::*;

#[elrond_wasm::module]
pub trait Validation: config::Config {
    fn require_is_accepted_payment_for_proposal(&self, payment: &EsdtTokenPayment<Self::Api>) {
        let governance_token_ids = self.governance_token_ids().get();

        let mut found = false;
        for token in governance_token_ids.iter() {
            if *token == payment.token_identifier {
                found = true;
                break;
            }
        }

        require!(found, BAD_TOKEN_ID);
    }

    fn require_is_accepted_payment_for_voting(&self, payment: &EsdtTokenPayment<Self::Api>) {
        let governance_token_ids = self.governance_token_ids().get();

        let mut found = false;
        for token in governance_token_ids.iter() {
            if *token == payment.token_identifier {
                found = true;
                break;
            }
        }

        require!(found, BAD_TOKEN_ID);
    }
}
