use common_structs::whitelist::Whitelist;

elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait WhitelistModule {
    #[only_owner]
    #[endpoint(addProxyToWhitelist)]
    fn add_proxy_to_whitelist(&self, address: ManagedAddress) {
        self.proxy_whitelist().add(&address);
    }

    #[only_owner]
    #[endpoint(removeProxyFromWhitelist)]
    fn remove_proxy_from_whitelist(&self, address: ManagedAddress) {
        self.proxy_whitelist().remove(&address);
    }

    #[only_owner]
    #[endpoint(addRewardsDepositorToWhitelist)]
    fn add_rewards_depositor_to_whitelist(&self, address: ManagedAddress) {
        self.rewards_depositor_whitelist().add(&address);
    }

    #[only_owner]
    #[endpoint(removeRewardsDepositorFromWhitelist)]
    fn remove_rewards_depositor_from_whitelist(&self, address: ManagedAddress) {
        self.rewards_depositor_whitelist().remove(&address);
    }

    #[only_owner]
    #[endpoint(addRewardTokenToWhitelist)]
    fn add_reward_token_to_whitelist(&self, token_id: TokenIdentifier) {
        self.reward_token_whitelist().add(&token_id);
    }

    #[only_owner]
    #[endpoint(removeRewardTokenFromWhitelist)]
    fn remove_reward_token_from_whitelist(&self, token_id: TokenIdentifier) {
        self.reward_token_whitelist().remove(&token_id);
    }

    #[storage_mapper("proxyWhitelist")]
    fn proxy_whitelist(&self) -> Whitelist<Self::Api, ManagedAddress>;

    #[storage_mapper("rewardsDepositorWhitelist")]
    fn rewards_depositor_whitelist(&self) -> Whitelist<Self::Api, ManagedAddress>;

    #[storage_mapper("rewardTokenWhitelist")]
    fn reward_token_whitelist(&self) -> Whitelist<Self::Api, TokenIdentifier>;
}
