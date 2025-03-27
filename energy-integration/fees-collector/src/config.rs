multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ConfigModule {
    #[inline(always)]
    fn add_known_token(&self, token_id: TokenIdentifier) {
        let _ = self.all_known_tokens().insert(token_id);
    }

    #[view(getAllTokens)]
    #[storage_mapper("allKnownTokens")]
    fn all_known_tokens(&self) -> UnorderedSetMapper<TokenIdentifier>;

    #[storage_mapper("allAccTokens")]
    fn all_accumulated_tokens(&self, token_id: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    // Update for this storage disabled for this version of the exchange
    #[view(getAllowExternalClaimRewards)]
    #[storage_mapper("allowExternalClaimRewards")]
    fn allow_external_claim_rewards(&self, user: &ManagedAddress) -> SingleValueMapper<bool>;
}
