#![no_std]
#![allow(clippy::type_complexity)]

use common_structs::{UnlockMilestone, UnlockPeriod};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod global_op;

const GAS_THRESHOLD: u64 = 100_000;
const MAX_CLAIMABLE_DISTRIBUTION_ROUNDS: usize = 4;

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
pub trait Distribution: global_op::GlobalOperationModule {
    #[proxy]
    fn locked_asset_factory_proxy(&self, to: ManagedAddress) -> factory::Proxy<Self::Api>;

    #[init]
    fn init(&self, asset_token_id: TokenIdentifier, locked_asset_factory_address: ManagedAddress) {
        require!(
            asset_token_id.is_valid_esdt_identifier(),
            "Asset token ID is not a valid esdt identifier"
        );

        self.asset_token_id().set_if_empty(&asset_token_id);
        self.locked_asset_factory_address()
            .set_if_empty(&locked_asset_factory_address);
    }

    #[only_owner]
    #[endpoint(setCommunityDistribution)]
    fn set_community_distribution(&self, total_amount: BigUint, spread_epoch: u64) {
        self.require_global_op_ongoing();
        require!(total_amount > 0, "Zero amount");
        require!(
            spread_epoch >= self.blockchain().get_block_epoch(),
            "Spread epoch in the past"
        );
        require!(
            self.community_distribution_list()
                .front()
                .map(|community_distrib| community_distrib.get_value_as_ref().spread_epoch)
                .unwrap_or_default()
                < spread_epoch,
            "Community distribution should be added in chronological order"
        );

        let distrib = CommunityDistribution {
            total_amount: total_amount.clone(),
            spread_epoch,
            after_planning_amount: total_amount,
        };
        self.community_distribution_list().push_front(distrib);
    }

    #[only_owner]
    #[endpoint(setPerUserDistributedLockedAssets)]
    fn set_per_user_distributed_locked_assets(
        &self,
        spread_epoch: u64,
        user_locked_assets: MultiValueEncoded<MultiValue2<ManagedAddress, BigUint>>,
    ) {
        self.require_global_op_ongoing();
        self.require_community_distribution_list_not_empty();

        require!(!user_locked_assets.is_empty(), "Empty assets vec");
        self.add_all_user_assets_to_map(spread_epoch, user_locked_assets)
    }

    #[endpoint(claimLockedAssets)]
    fn claim_locked_assets(&self) -> BigUint {
        self.require_global_op_not_ongoing();
        self.require_unlock_period_not_empty();
        self.require_community_distribution_list_not_empty();

        let caller = self.blockchain().get_caller();
        let mut cummulated_amount = BigUint::zero();

        let locked_assets = self.calculate_user_locked_assets(&caller, true);
        if locked_assets.is_empty() {
            return cummulated_amount;
        }

        let to = self.locked_asset_factory_address().get();
        let gas_limit_per_execute =
            self.blockchain().get_gas_left() / (locked_assets.len() as u64 + 1);

        let unlock_period = self.unlock_period().get();
        for elem in locked_assets.iter() {
            let amount = elem.biguint;
            let spread_epoch = elem.epoch;
            let _: IgnoreValue = self
                .locked_asset_factory_proxy(to.clone())
                .create_and_forward_custom_period(
                    amount.clone(),
                    caller.clone(),
                    spread_epoch,
                    unlock_period.clone(),
                )
                .with_gas_limit(gas_limit_per_execute)
                .execute_on_dest_context();

            cummulated_amount += amount;
        }

        cummulated_amount
    }

    #[endpoint(clearUnclaimableAssets)]
    fn clear_unclaimable_assets(&self) -> usize {
        let biggest_unclaimable_asset_epoch = self.get_biggest_unclaimable_asset_epoch();
        self.undo_user_assets_between_epochs(0, biggest_unclaimable_asset_epoch)
    }

    #[only_owner]
    #[endpoint(undoLastCommunityDistribution)]
    fn undo_last_community_distrib(&self) {
        self.require_global_op_ongoing();
        self.require_community_distribution_list_not_empty();
        self.community_distribution_list().pop_front();
    }

    #[only_owner]
    #[endpoint(undoUserDistributedAssetsBetweenEpochs)]
    fn undo_user_assets_between_epochs(&self, lower: u64, higher: u64) -> usize {
        self.require_global_op_ongoing();
        self.require_community_distribution_list_not_empty();
        require!(lower <= higher, "Bad input values");
        self.remove_asset_entries_between_epochs(lower, higher)
    }

    #[only_owner]
    #[endpoint(setUnlockPeriod)]
    fn set_unlock_period(&self, milestones: MultiValueEncoded<UnlockMilestone>) {
        let unlock_milestones = milestones.to_vec();
        self.validate_unlock_milestones(&unlock_milestones);
        self.unlock_period()
            .set(&UnlockPeriod { unlock_milestones });
    }

    #[view(calculateLockedAssets)]
    fn calculate_locked_assets_view(&self, address: ManagedAddress) -> BigUint {
        self.require_global_op_not_ongoing();
        self.require_community_distribution_list_not_empty();
        let locked_assets = self.calculate_user_locked_assets(&address, false);

        let mut cummulated_amount = BigUint::zero();
        for elem in locked_assets.iter() {
            cummulated_amount += elem.biguint;
        }
        cummulated_amount
    }

    fn validate_unlock_milestones(&self, unlock_milestones: &ManagedVec<UnlockMilestone>) {
        require!(!unlock_milestones.is_empty(), "Empty param");

        let mut percents_sum: u8 = 0;
        let mut last_milestone_unlock_epoch: u64 = 0;

        for milestone in unlock_milestones.into_iter() {
            require!(
                milestone.unlock_epoch >= last_milestone_unlock_epoch,
                "Unlock epochs not in order"
            );
            require!(
                milestone.unlock_percent <= 100,
                "Unlock percent more than 100"
            );
            last_milestone_unlock_epoch = milestone.unlock_epoch;
            percents_sum += milestone.unlock_percent;
        }

        require!(percents_sum == 100, "Percents do not sum up to 100");
    }

    fn add_all_user_assets_to_map(
        &self,
        spread_epoch: u64,
        user_assets: MultiValueEncoded<MultiValue2<ManagedAddress, BigUint>>,
    ) {
        let mut last_community_distrib = self
            .community_distribution_list()
            .front()
            .unwrap()
            .get_value_cloned();
        require!(
            spread_epoch == last_community_distrib.spread_epoch,
            "Bad spread epoch"
        );

        for user_asset_multiarg in user_assets.into_iter() {
            let (caller, asset_amount) = user_asset_multiarg.into_tuple();
            require!(asset_amount > 0, "Zero amount");
            require!(
                last_community_distrib.after_planning_amount >= asset_amount,
                "User assets sums above community total assets"
            );
            last_community_distrib.after_planning_amount -= &asset_amount;
            self.add_user_locked_asset_entry(caller, asset_amount, spread_epoch);
        }

        self.community_distribution_list().pop_front();
        self.community_distribution_list()
            .push_front(last_community_distrib);
    }

    fn add_user_locked_asset_entry(
        &self,
        caller: ManagedAddress,
        asset_amount: BigUint,
        spread_epoch: u64,
    ) {
        let key = UserLockedAssetKey {
            caller,
            spread_epoch,
        };
        require!(
            !self.user_locked_asset_map().contains_key(&key),
            "Vector has duplicates"
        );
        self.user_locked_asset_map().insert(key, asset_amount);
    }

    fn calculate_user_locked_assets(
        &self,
        address: &ManagedAddress,
        delete_after_visit: bool,
    ) -> ManagedVec<BigUintEpochPair<Self::Api>> {
        let current_epoch = self.blockchain().get_block_epoch();
        let mut locked_assets = ManagedVec::new();

        for community_distrib in self
            .community_distribution_list()
            .iter()
            .take(MAX_CLAIMABLE_DISTRIBUTION_ROUNDS)
            .filter(|x| x.get_value_as_ref().spread_epoch <= current_epoch)
        {
            let user_asset_key = UserLockedAssetKey {
                caller: address.clone(),
                spread_epoch: community_distrib.get_value_as_ref().spread_epoch,
            };

            if let Some(asset_amount) = self.user_locked_asset_map().get(&user_asset_key) {
                locked_assets.push(BigUintEpochPair {
                    biguint: asset_amount,
                    epoch: user_asset_key.spread_epoch,
                });

                if delete_after_visit {
                    self.user_locked_asset_map().remove(&user_asset_key);
                }
            }
        }
        locked_assets
    }

    fn get_biggest_unclaimable_asset_epoch(&self) -> u64 {
        self.community_distribution_list()
            .iter()
            .nth(MAX_CLAIMABLE_DISTRIBUTION_ROUNDS)
            .map(|community_distrib| community_distrib.get_value_as_ref().spread_epoch)
            .unwrap_or_default()
    }

    fn remove_asset_entries_between_epochs(&self, lower: u64, higher: u64) -> usize {
        if higher == 0 {
            return 0;
        }

        if higher < lower {
            return 0;
        }

        let mut to_remove_keys = ManagedVec::<Self::Api, UserLockedAssetKey<Self::Api>>::new();
        let search_gas_limit = self.blockchain().get_gas_left() / 2;
        for user_asset_key in self.user_locked_asset_map().keys() {
            if self.blockchain().get_gas_left() < search_gas_limit {
                break;
            }

            if lower <= user_asset_key.spread_epoch && user_asset_key.spread_epoch <= higher {
                to_remove_keys.push(user_asset_key);
            }
        }

        let map_len_before = self.user_locked_asset_map().len();
        for key in to_remove_keys.iter() {
            if self.blockchain().get_gas_left() < GAS_THRESHOLD {
                break;
            }

            self.user_locked_asset_map().remove(&key);
        }
        map_len_before - self.user_locked_asset_map().len()
    }

    fn require_community_distribution_list_not_empty(&self) {
        require!(
            !self.community_distribution_list().is_empty(),
            "Empty community assets list"
        );
    }

    fn require_unlock_period_not_empty(&self) {
        require!(!self.unlock_period().is_empty(), "Empty unlock schedule");
    }

    #[only_owner]
    #[endpoint(deleteUserDistributedLockedAssets)]
    fn delete_user_distributed_locked_assets(&self, spread_epoch: u64, address: ManagedAddress) {
        self.require_global_op_ongoing();
        self.user_locked_asset_map().remove(&UserLockedAssetKey {
            caller: address,
            spread_epoch,
        });
    }

    #[view(getUsersDistributedLockedAssetsLength)]
    fn get_users_distributed_locked_assets_length(&self) -> usize {
        self.user_locked_asset_map().len()
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
}
