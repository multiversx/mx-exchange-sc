elrond_wasm::imports!();

use crate::{config, proposal::ProposalCreationArgs};

#[elrond_wasm::module]
pub trait Validation: config::Config {
    fn require_is_accepted_payment_for_proposal(&self, _payment: &EsdtTokenPayment<Self::Api>) {
        todo!()
    }

    fn require_is_accepted_payment_for_voting(&self, _payment: &EsdtTokenPayment<Self::Api>) {
        todo!()
    }

    fn require_are_accepted_args_for_proposal(&self, _args: &ProposalCreationArgs<Self::Api>) {
        todo!()
    }
}
