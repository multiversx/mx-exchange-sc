multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use super::base::SwapTokensOrder;

pub struct SwapContext<M: ManagedTypeApi> {
    pub input_token_id: TokenIdentifier<M>,
    pub input_token_amount: BigUint<M>,

    pub output_token_id: TokenIdentifier<M>,
    pub output_token_amount: BigUint<M>,
    pub swap_tokens_order: SwapTokensOrder,

    pub final_input_amount: BigUint<M>,
    pub final_output_amount: BigUint<M>,
    pub fee_amount: BigUint<M>,
}

impl<M: ManagedTypeApi> SwapContext<M> {
    pub fn new(
        input_token_id: TokenIdentifier<M>,
        input_token_amount: BigUint<M>,
        output_token_id: TokenIdentifier<M>,
        output_token_amount: BigUint<M>,
        swap_tokens_order: SwapTokensOrder,
    ) -> Self {
        SwapContext {
            input_token_id,
            input_token_amount,
            output_token_id,
            output_token_amount,
            swap_tokens_order,
            final_input_amount: BigUint::zero(),
            final_output_amount: BigUint::zero(),
            fee_amount: BigUint::zero(),
        }
    }
}
