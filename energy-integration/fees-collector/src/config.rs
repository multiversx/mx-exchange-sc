multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ConfigModule {
    fn add_known_token(&self, token_id: &TokenIdentifier) {
        let known_tokens_mapper = self.known_tokens();
        if known_tokens_mapper.contains(&token_id) {
            return;
        }

        known_tokens_mapper.add(&token_id);
        self.all_tokens().update(|all_tokens| {
            all_tokens.push(token_id.clone());
        });
    }

    #[view(getAllTokens)]
    fn get_all_tokens(&self) -> MultiValueEncoded<TokenIdentifier> {
        self.all_tokens().get().into()
    }

    #[storage_mapper("knownTokens")]
    fn known_tokens(&self) -> WhitelistMapper<TokenIdentifier>;

    #[storage_mapper("allTokens")]
    fn all_tokens(&self) -> SingleValueMapper<ManagedVec<TokenIdentifier>>;

    // Update for this storage disabled for this version of the exchange
    #[view(getAllowExternalClaimRewards)]
    #[storage_mapper("allowExternalClaimRewards")]
    fn allow_external_claim_rewards(&self, user: &ManagedAddress) -> SingleValueMapper<bool>;
}
