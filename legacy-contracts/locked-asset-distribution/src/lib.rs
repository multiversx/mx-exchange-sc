#![no_std]
#![allow(clippy::type_complexity)]

use common_structs::UnlockPeriod;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(ManagedVecItem)]
pub struct BigUintEpochPair<M: ManagedTypeApi> {
    pub biguint: BigUint<M>,
    pub epoch: u64,
}

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi)]
pub struct UserLockedAssetKey<M: ManagedTypeApi> {
    pub caller: ManagedAddress<M>,
    pub spread_epoch: u64,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi, Clone)]
pub struct CommunityDistribution<M: ManagedTypeApi> {
    pub total_amount: BigUint<M>,
    pub spread_epoch: u64,
    pub after_planning_amount: BigUint<M>,
}

#[multiversx_sc::contract]
pub trait Distribution {
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[endpoint(clearSingleValueMappers)]
    fn clear_single_value_mappers(&self) {
        self.unlock_period().clear();
        self.locked_asset_factory_address().clear();
        self.asset_token_id().clear();
        self.global_op_is_ongoing().clear();
    }

    #[endpoint(clearCommunityDistributionList)]
    fn clear_community_distribution_list(&self, n: u64) {
        for _ in 0..n {
            self.community_distribution_list().pop_front();
        }
    }

    #[endpoint(clearUserLockedAssetMap)]
    fn clear_user_locked_asset_map(&self, n: u64) {
        for _ in 0..n {
            let key = self.user_locked_asset_map().keys().last().unwrap();
            self.user_locked_asset_map().remove(&key);
        }
    }

    #[view(getUnlockPeriod)]
    #[storage_mapper("unlock_period")]
    fn unlock_period(&self) -> SingleValueMapper<UnlockPeriod<Self::Api>>;

    #[view(getCommunityDistributionList)]
    #[storage_mapper("community_distribution_list")]
    fn community_distribution_list(&self) -> LinkedListMapper<CommunityDistribution<Self::Api>>;

    #[storage_mapper("user_locked_asset_map")]
    fn user_locked_asset_map(&self) -> MapMapper<UserLockedAssetKey<Self::Api>, BigUint>;

    #[storage_mapper("locked_asset_factory_address")]
    fn locked_asset_factory_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getAssetTokenId)]
    #[storage_mapper("asset_token_id")]
    fn asset_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("global_operation_ongoing")]
    fn global_op_is_ongoing(&self) -> SingleValueMapper<bool>;
}
