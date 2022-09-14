elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait TokenWhitelistModule {
    #[only_owner]
    #[endpoint(addTokensToWhitelist)]
    fn add_tokens_to_whitelist(&self, tokens: MultiValueEncoded<TokenIdentifier>) {
        let mut whitelist = self.token_whitelist();
        for token_id in tokens {
            self.require_has_transfer_role_for_token(&token_id);

            let _ = whitelist.insert(token_id);
        }
    }

    #[only_owner]
    #[endpoint(removeTokensFromWhitelist)]
    fn remove_tokens_from_whitelist(&self, tokens: MultiValueEncoded<TokenIdentifier>) {
        let mut whitelist = self.token_whitelist();
        for token_id in tokens {
            let _ = whitelist.swap_remove(&token_id);
        }
    }

    fn require_has_transfer_role_for_token(&self, token_id: &TokenIdentifier) {
        let roles = self.blockchain().get_esdt_local_roles(token_id);
        require!(
            roles.has_role(&EsdtLocalRole::Transfer),
            "SC does not have ESDT transfer role for {}",
            token_id
        );
    }

    fn require_token_in_whitelist(&self, token_id: &TokenIdentifier) {
        require!(
            self.token_whitelist().contains(token_id),
            "Token {} is not whitelisted",
            token_id
        );
    }

    #[view(getTokenWhitelist)]
    #[storage_mapper("tokenWhitelist")]
    fn token_whitelist(&self) -> UnorderedSetMapper<TokenIdentifier>;
}
