elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TopEncode)]
pub struct SwapEventAmounts<BigUint: BigUintApi> {
    amount_in: BigUint,
    fee_amount: BigUint,
    amount_out: BigUint,
}

#[derive(TopEncode)]
pub struct LiquidityChangeAmounts<BigUint: BigUintApi> {
    first_token_amount: BigUint,
    second_token_amount: BigUint,
    liquidity: BigUint,
}

#[elrond_wasm_derive::module]
pub trait EventsModule {
    fn emit_swap_event(
        &self,
        token_in: &TokenIdentifier,
        token_out: &TokenIdentifier,
        amount_in: &Self::BigUint,
        fee_amount: &Self::BigUint,
        amount_out: &Self::BigUint,
    ) {
        self.swap_event(
            token_in,
            token_out,
            &self.blockchain().get_sc_address(),
            SwapEventAmounts {
                amount_in: amount_in.clone(),
                fee_amount: fee_amount.clone(),
                amount_out: amount_out.clone(),
            },
        )
    }

    fn emit_add_liquidity_event(
        &self,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
        first_token_amount: &Self::BigUint,
        second_token_amount: &Self::BigUint,
        liquidity: &Self::BigUint,
    ) {
        self.add_liquidity_event(
            first_token,
            second_token,
            &self.blockchain().get_sc_address(),
            LiquidityChangeAmounts {
                first_token_amount: first_token_amount.clone(),
                second_token_amount: second_token_amount.clone(),
                liquidity: liquidity.clone(),
            },
        )
    }

    fn emit_remove_liquidity_event(
        &self,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
        first_token_amount: &Self::BigUint,
        second_token_amount: &Self::BigUint,
        liquidity: &Self::BigUint,
    ) {
        self.remove_liquidity_event(
            first_token,
            second_token,
            &self.blockchain().get_sc_address(),
            LiquidityChangeAmounts {
                first_token_amount: first_token_amount.clone(),
                second_token_amount: second_token_amount.clone(),
                liquidity: liquidity.clone(),
            },
        )
    }

    #[event("swap")]
    fn swap_event(
        &self,
        #[indexed] token_in: &TokenIdentifier,
        #[indexed] token_out: &TokenIdentifier,
        #[indexed] sc_address: &Address,
        swap_amounts: SwapEventAmounts<Self::BigUint>,
    );

    #[event("add_liquidity")]
    fn add_liquidity_event(
        &self,
        #[indexed] first_token: &TokenIdentifier,
        #[indexed] second_token: &TokenIdentifier,
        #[indexed] sc_address: &Address,
        liquidity_change: LiquidityChangeAmounts<Self::BigUint>,
    );

    #[event("remove_liquidity")]
    fn remove_liquidity_event(
        &self,
        #[indexed] first_token: &TokenIdentifier,
        #[indexed] second_token: &TokenIdentifier,
        #[indexed] sc_address: &Address,
        liquidity_change: LiquidityChangeAmounts<Self::BigUint>,
    );
}
