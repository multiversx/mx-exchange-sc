use multiversx_sc_scenario::{
    api::StaticApi,
    imports::{
        BigUint, EsdtTokenPayment, ManagedTypeApi, ManagedVec, RustBigUint, TokenIdentifier,
    },
};

use crate::{
    dex_interact_cli::{AddArgs, SwapArgs},
    DexInteract,
};

pub struct InteractorToken {
    pub token_id: String,
    pub nonce: u64,
    pub amount: RustBigUint,
}

impl<M: ManagedTypeApi> From<EsdtTokenPayment<M>> for InteractorToken {
    fn from(value: EsdtTokenPayment<M>) -> Self {
        InteractorToken {
            token_id: value.token_identifier.to_string(),
            nonce: value.token_nonce,
            amount: RustBigUint::from_bytes_be(value.amount.to_bytes_be().as_slice()),
        }
    }
}

impl<M: ManagedTypeApi> From<InteractorToken> for EsdtTokenPayment<M> {
    fn from(interactor_token: InteractorToken) -> Self {
        EsdtTokenPayment::new(
            TokenIdentifier::from(interactor_token.token_id.as_bytes()),
            interactor_token.nonce,
            BigUint::from(interactor_token.amount),
        )
    }
}

impl<M: ManagedTypeApi> From<&InteractorToken> for EsdtTokenPayment<M> {
    fn from(interactor_token: &InteractorToken) -> Self {
        EsdtTokenPayment::new(
            TokenIdentifier::from(interactor_token.token_id.as_bytes()),
            interactor_token.nonce,
            BigUint::from(interactor_token.amount.clone()),
        )
    }
}

impl AddArgs {
    pub fn as_payment_vec(
        &self,
        dex_interact: &mut DexInteract,
    ) -> ManagedVec<StaticApi, EsdtTokenPayment<StaticApi>> {
        let first_token_id = dex_interact.state.first_token_id().as_bytes();
        let second_token_id = dex_interact.state.second_token_id().as_bytes();

        let mut payments = ManagedVec::from_single_item(EsdtTokenPayment::new(
            TokenIdentifier::from(first_token_id),
            0,
            BigUint::from(self.first_payment_amount),
        ));
        payments.push(EsdtTokenPayment::new(
            TokenIdentifier::from(second_token_id),
            0,
            BigUint::from(self.second_payment_amount),
        ));
        payments
    }
}

impl SwapArgs {
    pub fn as_payment(&self, dex_interact: &mut DexInteract) -> EsdtTokenPayment<StaticApi> {
        let first_token_id = dex_interact.state.first_token_id().as_bytes();
        EsdtTokenPayment::new(
            TokenIdentifier::from(first_token_id),
            0,
            BigUint::from(self.amount),
        )
    }
}
