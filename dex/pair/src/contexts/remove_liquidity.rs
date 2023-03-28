multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub struct RemoveLiquidityContext<M: ManagedTypeApi> {
    pub lp_token_payment_amount: BigUint<M>,
    pub first_token_amount_min: BigUint<M>,
    pub second_token_amount_min: BigUint<M>,
    pub first_token_amount_removed: BigUint<M>,
    pub second_token_amount_removed: BigUint<M>,
}

impl<M: ManagedTypeApi> RemoveLiquidityContext<M> {
    pub fn new(
        lp_token_payment_amount: BigUint<M>,
        first_token_amount_min: BigUint<M>,
        second_token_amount_min: BigUint<M>,
    ) -> Self {
        RemoveLiquidityContext {
            lp_token_payment_amount,
            first_token_amount_min,
            second_token_amount_min,
            first_token_amount_removed: BigUint::zero(),
            second_token_amount_removed: BigUint::zero(),
        }
    }
}
