multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub struct AddLiquidityContext<M: ManagedTypeApi> {
    pub first_payment: EsdtTokenPayment<M>,
    pub second_payment: EsdtTokenPayment<M>,
    pub first_token_amount_min: BigUint<M>,
    pub second_token_amount_min: BigUint<M>,
    pub first_token_optimal_amount: BigUint<M>,
    pub second_token_optimal_amount: BigUint<M>,
    pub liq_added: BigUint<M>,
}

impl<M: ManagedTypeApi> AddLiquidityContext<M> {
    pub fn new(
        first_payment: EsdtTokenPayment<M>,
        second_payment: EsdtTokenPayment<M>,
        first_token_amount_min: BigUint<M>,
        second_token_amount_min: BigUint<M>,
    ) -> Self {
        AddLiquidityContext {
            first_payment,
            second_payment,
            first_token_amount_min,
            second_token_amount_min,
            first_token_optimal_amount: BigUint::zero(),
            second_token_optimal_amount: BigUint::zero(),
            liq_added: BigUint::zero(),
        }
    }
}
