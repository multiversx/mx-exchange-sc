elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub type AddLiquidityResultType<M> =
    MultiValue3<EsdtTokenPayment<M>, EsdtTokenPayment<M>, EsdtTokenPayment<M>>;
pub type RemoveLiquidityResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

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
pub mod lp_proxy {
    elrond_wasm::imports!();
    use super::{AddLiquidityResultType, RemoveLiquidityResultType};

    #[elrond_wasm::proxy]
    pub trait LpProxy {
        #[payable("*")]
        #[endpoint(addLiquidity)]
        fn add_liquidity(
            &self,
            first_token_amount_min: BigUint,
            second_token_amount_min: BigUint,
        ) -> AddLiquidityResultType<Self::Api>;

        #[payable("*")]
        #[endpoint(removeLiquidity)]
        fn remove_liquidity(
            &self,
            first_token_amount_min: BigUint,
            second_token_amount_min: BigUint,
        ) -> RemoveLiquidityResultType<Self::Api>;
    }
}

#[elrond_wasm::module]
pub trait LpInteractionsModule {
    fn call_pair_add_liquidity(
        &self,
        lp_address: ManagedAddress,
        first_payment: EsdtTokenPayment<Self::Api>,
        second_payment: EsdtTokenPayment<Self::Api>,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> AddLiquidityResultWrapper<Self::Api> {
        let second_token_id_copy = second_payment.token_identifier.clone();

        let mut lp_payments_in = ManagedVec::new();
        lp_payments_in.push(first_payment);
        lp_payments_in.push(second_payment);

        let lp_payments_out: AddLiquidityResultType<Self::Api> = self
            .lp_proxy(lp_address)
            .add_liquidity(first_token_amount_min, second_token_amount_min)
            .with_multi_token_transfer(lp_payments_in)
            .execute_on_dest_context_custom_range(|_, after| (after - 3, after));
        let (lp_tokens, mut first_token_refund, mut second_token_refund) =
            lp_payments_out.into_tuple();

        if first_token_refund.token_identifier == second_token_id_copy {
            core::mem::swap(&mut first_token_refund, &mut second_token_refund);
        }

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
        expected_first_token_id_out: TokenIdentifier,
    ) -> RemoveLiquidityResultWrapper<Self::Api> {
        let lp_payments_out: RemoveLiquidityResultType<Self::Api> = self
            .lp_proxy(lp_address)
            .remove_liquidity(first_token_amount_min, second_token_amount_min)
            .add_token_transfer(lp_token_id, 0, lp_token_amount)
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after));

        let (mut first_token_payment_out, mut second_token_payment_out) =
            lp_payments_out.into_tuple();

        if second_token_payment_out.token_identifier == expected_first_token_id_out {
            core::mem::swap(&mut first_token_payment_out, &mut second_token_payment_out);
        }

        RemoveLiquidityResultWrapper {
            first_token_payment_out,
            second_token_payment_out,
        }
    }

    #[proxy]
    fn lp_proxy(&self, sc_address: ManagedAddress) -> lp_proxy::Proxy<Self::Api>;
}
