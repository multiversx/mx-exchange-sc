use multiversx_sc_scenario::imports::{BigUint, ManagedVec, RustBigUint, TokenIdentifier};
use multiversx_sc_scenario::imports::{EsdtTokenPayment, ManagedTypeApi, ReturnsResult};
use multiversx_sc_snippets::InteractorPrepareAsync;

use crate::DexInteract;
use proxies::pair_proxy;

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
        min_amount_to_receive: u128,
    ) -> InteractorToken {
        let first_token_id = self.state.first_token_id();
        let second_token_id = self.state.second_token_id();

        println!(
            "Attempting to swap {amount_to_swap} {first_token_id} for a min amount {min_amount_to_receive} of {second_token_id}..."
        );

        let result_token = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_pair_address())
            .gas(100_000_000u64)
            .typed(pair_proxy::PairProxy)
            .swap_tokens_fixed_input(
                TokenIdentifier::from(second_token_id.as_bytes()),
                BigUint::from(min_amount_to_receive),
            )
            .payment((
                TokenIdentifier::from(first_token_id.as_bytes()),
                0,
                BigUint::from(amount_to_swap),
            ))
            .returns(ReturnsResult)
            .prepare_async()
            .run()
            .await;

        println!("Result token {:#?}", result_token);

        InteractorToken::from(result_token)
    }

    pub async fn add_liquidity(
        &mut self,
        first_payment_amount: u128,
        second_payment_amount: u128,
        first_token_amount_min: u128,
        second_token_amount_min: u128,
    ) -> (InteractorToken, InteractorToken, InteractorToken) {
        println!("Attempting to add liquidity to pair...");
        let first_token_id = self.state.first_token_id().as_bytes();
        let second_token_id = self.state.second_token_id().as_bytes();

        let mut payments = ManagedVec::from_single_item(EsdtTokenPayment::new(
            TokenIdentifier::from(first_token_id),
            0,
            BigUint::from(first_payment_amount),
        ));
        payments.push(EsdtTokenPayment::new(
            TokenIdentifier::from(second_token_id),
            0,
            BigUint::from(second_payment_amount),
        ));

        let result_token = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_pair_address())
            .gas(100_000_000u64)
            .typed(pair_proxy::PairProxy)
            .add_liquidity(
                BigUint::from(first_token_amount_min),
                BigUint::from(second_token_amount_min),
            )
            .payment(payments)
            .returns(ReturnsResult)
            .prepare_async()
            .run()
            .await;
        (
            InteractorToken::from(result_token.0 .0),
            InteractorToken::from(result_token.0 .1),
            InteractorToken::from(result_token.0 .2),
        )
    }
}

// 10000000000000000000 ; 10 UTK
// 1000000000000; 0,000001 WEGLD
// cargo run swap -a 10000000000000000000 -m 1000000000000
