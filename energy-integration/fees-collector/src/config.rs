multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ConfigModule: energy_query::EnergyQueryModule {
    #[only_owner]
    #[endpoint(addRewardTokens)]
    fn add_reward_tokens(&self, token_ids: MultiValueEncoded<TokenIdentifier>) {
        for token_id in token_ids {
            require!(self.reward_tokens().insert(token_id), "Token already added");
        }
    }

    #[only_owner]
    #[endpoint(removeRewardTokens)]
    fn remove_reward_tokens(&self, token_ids: MultiValueEncoded<TokenIdentifier>) {
        for token_id in token_ids {
            require!(
                self.reward_tokens().swap_remove(&token_id),
                "Token not found"
            );
        }
    }

    fn set_base_reward_tokens(&self) {
        let locked_token_id = self.get_locked_token_id();
        let base_token_id = self.get_base_token_id();

        let mut base_reward_tokens = MultiValueEncoded::new();
        base_reward_tokens.push(locked_token_id);
        base_reward_tokens.push(base_token_id);

        self.add_reward_tokens(base_reward_tokens);
    }

    #[view(getRewardTokens)]
    #[storage_mapper("rewardTokens")]
    fn reward_tokens(&self) -> UnorderedSetMapper<TokenIdentifier>;

    // Update for this storage disabled for this version of the exchange
    #[view(getAllowExternalClaimRewards)]
    #[storage_mapper("allowExternalClaimRewards")]
    fn allow_external_claim_rewards(&self, user: &ManagedAddress) -> SingleValueMapper<bool>;
}
