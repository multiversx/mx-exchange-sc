elrond_wasm::imports!();

pub const MAX_PERCENTAGE: u64 = 10_000_000_000_000; // 100%

#[elrond_wasm::module]
pub trait CommonStorageModule {
    #[view(getLaunchedTokenId)]
    #[storage_mapper("launchedTokenId")]
    fn launched_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getAcceptedTokenId)]
    #[storage_mapper("acceptedTokenId")]
    fn accepted_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLpTokenId)]
    #[storage_mapper("lpTokenId")]
    fn lp_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLaunchedTokenFinalAmount)]
    #[storage_mapper("launchedTokenFinalAmount")]
    fn launched_token_final_amount(&self) -> SingleValueMapper<BigUint>;

    #[view(getAcceptedTokenFinalAmount)]
    #[storage_mapper("acceptedTokenFinalAmount")]
    fn accepted_token_final_amount(&self) -> SingleValueMapper<BigUint>;

    #[view(totalLpTokensReceived)]
    #[storage_mapper("totalLpTokensReceived")]
    fn total_lp_tokens_received(&self) -> SingleValueMapper<BigUint>;

    #[view(getStartBlock)]
    #[storage_mapper("startBlock")]
    fn start_block(&self) -> SingleValueMapper<u64>;

    #[view(getEndBlock)]
    #[storage_mapper("endBlock")]
    fn end_block(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("accumulatedPenalty")]
    fn accumulated_penalty(&self, redeem_token_nonce: u64) -> SingleValueMapper<BigUint>;
}
