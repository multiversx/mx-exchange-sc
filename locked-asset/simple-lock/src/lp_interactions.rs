multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{error_messages::*, lp_proxy};

pub struct AddLiquidityResultWrapper<M: ManagedTypeApi> {
    pub lp_tokens: EsdtTokenPayment<M>,
    pub first_token_refund: EsdtTokenPayment<M>,
    pub second_token_refund: EsdtTokenPayment<M>,
}

pub struct RemoveLiquidityResultWrapper<M: ManagedTypeApi> {
    pub first_token_payment_out: EsdtTokenPayment<M>,
    pub second_token_payment_out: EsdtTokenPayment<M>,
}

// Must manually declare, as Pair SC already depends on simple-lock
// This avoids circular dependency
#[multiversx_sc::module]
pub trait LpInteractionsModule {
    fn call_pair_add_liquidity(
        &self,
        lp_address: ManagedAddress,
        first_payment: &EsdtTokenPayment<Self::Api>,
        second_payment: &EsdtTokenPayment<Self::Api>,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> AddLiquidityResultWrapper<Self::Api> {
        let mut lp_payments_in = ManagedVec::new();
        lp_payments_in.push(first_payment.clone());
        lp_payments_in.push(second_payment.clone());

        let lp_payments_out = self
            .tx()
            .to(&lp_address)
            .typed(lp_proxy::LpProxy)
            .add_liquidity(first_token_amount_min, second_token_amount_min)
            .payment(lp_payments_in)
            .returns(ReturnsResult)
            .sync_call();

        let (lp_tokens, first_token_optimal_payment, second_token_optimal_payment) =
            lp_payments_out.into_tuple();

        require!(
            first_payment.token_identifier == first_token_optimal_payment.token_identifier
                && second_payment.token_identifier == second_token_optimal_payment.token_identifier,
            INVALID_PAYMENTS_RECEIVED_FROM_LP_ERR_MSG
        );

        let first_refund_amount = &first_payment.amount - &first_token_optimal_payment.amount;
        let first_token_refund = EsdtTokenPayment::new(
            first_token_optimal_payment.token_identifier,
            0,
            first_refund_amount,
        );

        let second_refund_amount = &second_payment.amount - &second_token_optimal_payment.amount;
        let second_token_refund = EsdtTokenPayment::new(
            second_token_optimal_payment.token_identifier,
            0,
            second_refund_amount,
        );

        AddLiquidityResultWrapper {
            lp_tokens,
            first_token_refund,
            second_token_refund,
        }
    }

    fn call_pair_remove_liquidity(
        &self,
        lp_address: ManagedAddress,
        lp_token_id: TokenIdentifier,
        lp_token_amount: BigUint,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
        expected_first_token_id_out: &TokenIdentifier,
        expected_second_token_id_out: &TokenIdentifier,
    ) -> RemoveLiquidityResultWrapper<Self::Api> {
        let lp_payments_out = self
            .tx()
            .to(&lp_address)
            .typed(lp_proxy::LpProxy)
            .remove_liquidity(first_token_amount_min, second_token_amount_min)
            .single_esdt(&lp_token_id, 0, &lp_token_amount)
            .returns(ReturnsResult)
            .sync_call();

        let (first_token_payment_out, second_token_payment_out) = lp_payments_out.into_tuple();
        require!(
            &first_token_payment_out.token_identifier == expected_first_token_id_out
                && &second_token_payment_out.token_identifier == expected_second_token_id_out,
            INVALID_PAYMENTS_RECEIVED_FROM_LP_ERR_MSG
        );

        RemoveLiquidityResultWrapper {
            first_token_payment_out,
            second_token_payment_out,
        }
    }
}
