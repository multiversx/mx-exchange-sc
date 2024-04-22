use multiversx_sc_scenario::imports::{BigUint, RustBigUint, TokenIdentifier};
use multiversx_sc_scenario::imports::{EsdtTokenPayment, ManagedTypeApi, ReturnsResult};
use multiversx_sc_snippets::InteractorPrepareAsync;

use crate::pair_proxy;
use crate::DexInteract;

pub const UTK: &str = "UTK-14d57d";
pub const _WEGLD: &str = "WEGLD-a28c59";
pub const GAS: u64 = 100_000_000u64;

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

impl DexInteract {
    pub async fn swap_tokens_fixed_input(
        &mut self,
        amount_to_swap: u128,
        token_id: &str,
        min_amount_to_receive: u128,
    ) -> InteractorToken {
        println!(
            "Attempting to swap {amount_to_swap} {UTK} for a min amount {min_amount_to_receive} of {token_id}..."
        );

        let result_token = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_pair_address())
            .gas(GAS)
            .typed(pair_proxy::PairProxy)
            .swap_tokens_fixed_input(
                TokenIdentifier::from(token_id),
                BigUint::from(min_amount_to_receive),
            )
            .payment((TokenIdentifier::from(UTK), 0, BigUint::from(amount_to_swap)))
            .returns(ReturnsResult)
            .prepare_async()
            .run()
            .await;

        println!("Result token {:#?}", result_token);

        InteractorToken::from(result_token)
    }
}

// 10000000000000000000 ; 10 UTK
// 1000000000000; 0,000001 WEGLD
// cargo run swap -a 10000000000000000000 -t WEGLD-a28c59 -m 1000000000000
