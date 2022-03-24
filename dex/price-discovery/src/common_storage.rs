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

    #[view(getExtraRewardsTokenId)]
    #[storage_mapper("extraRewardsTokenId")]
    fn extra_rewards_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getExtraRewardsTokenNonce)]
    #[storage_mapper("extraRewardsTokenNonce")]
    fn extra_rewards_token_nonce(&self) -> SingleValueMapper<u64>;

    #[view(getLpTokenId)]
    #[storage_mapper("lpTokenId")]
    fn lp_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLaunchedTokenBalance)]
    #[storage_mapper("launchedTokenBalance")]
    fn launched_token_balance(&self) -> SingleValueMapper<BigUint>;

    #[view(getAcceptedTokenBalance)]
    #[storage_mapper("acceptedTokenBalance")]
    fn accepted_token_balance(&self) -> SingleValueMapper<BigUint>;

    #[view(getExtraRewardsBalance)]
    #[storage_mapper("extraRewardsBalance")]
    fn extra_rewards_balance(&self) -> SingleValueMapper<BigUint>;

    #[view(getTotalLpTokensReceived)]
    #[storage_mapper("totalLpTokensReceived")]
    fn total_lp_tokens_received(&self) -> SingleValueMapper<BigUint>;

    #[view(getTotalExtraRewardsTokens)]
    #[storage_mapper("totalExtraRewardsTokens")]
    fn total_extra_rewards_tokens(&self) -> SingleValueMapper<BigUint>;

    #[view(getStartBlock)]
    #[storage_mapper("startBlock")]
    fn start_block(&self) -> SingleValueMapper<u64>;

    #[view(getEndBlock)]
    #[storage_mapper("endBlock")]
    fn end_block(&self) -> SingleValueMapper<u64>;
}
