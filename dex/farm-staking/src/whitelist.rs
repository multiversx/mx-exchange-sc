elrond_wasm::imports!();

use core::marker::PhantomData;
use elrond_wasm::{api::StorageMapperApi, storage::StorageKey};

type FlagMapper<SA> = SingleValueMapper<SA, bool>;

pub struct Whitelist<SA, T>
where
    SA: StorageMapperApi,
    T: NestedEncode + 'static,
{
    base_key: StorageKey<SA>,
    _phantom: PhantomData<T>,
}

impl<SA, T> Whitelist<SA, T>
where
    SA: StorageMapperApi,
    T: NestedEncode + 'static,
{
    pub fn contains(&self, item: &T) -> bool {
        let mapper = self.build_mapper_for_item(item);
        !mapper.is_empty()
    }

    pub fn add(&mut self, item: &T) {
        let mapper = self.build_mapper_for_item(item);
        mapper.set(&true);
    }

    pub fn remove(&mut self, item: &T) {
        let mapper = self.build_mapper_for_item(item);
        mapper.clear();
    }

    pub fn require_whitelisted(&self, item: &T) {
        if !self.contains(item) {
            SA::error_api_impl().signal_error(b"Item not whitelisted");
        }
    }
}

impl<SA, T> Whitelist<SA, T>
where
    SA: StorageMapperApi,
    T: NestedEncode + 'static,
{
    fn build_mapper_for_item(&self, item: &T) -> FlagMapper<SA> {
        let mut key = self.base_key.clone();
        key.append_item(item);

        FlagMapper::<SA>::new(key)
    }
}

impl<SA, T> StorageMapper<SA> for Whitelist<SA, T>
where
    SA: StorageMapperApi,
    T: NestedEncode + 'static,
{
    fn new(base_key: StorageKey<SA>) -> Self {
        Self {
            base_key,
            _phantom: PhantomData,
        }
    }
}

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
