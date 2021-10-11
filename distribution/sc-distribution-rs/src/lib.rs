#![no_std]

use common_structs::{UnlockMilestone, UnlockPeriod};

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Epoch = u64;

mod global_op;

const GAS_THRESHOLD: u64 = 100_000;
const MAX_CLAIMABLE_DISTRIBUTION_ROUNDS: usize = 4;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi)]
pub struct UserLockedAssetKey {
    pub caller: Address,
    pub spread_epoch: u64,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct CommunityDistribution<BigUint: BigUintApi> {
    pub total_amount: BigUint,
    pub spread_epoch: u64,
    pub after_planning_amount: BigUint,
}

#[elrond_wasm::contract]
pub trait Distribution: global_op::GlobalOperationModule {
    #[proxy]
    fn locked_asset_factory_proxy(
        &self,
        to: Address,
    ) -> sc_locked_asset_factory::Proxy<Self::SendApi>;

    #[init]
    fn init(
        &self,
        asset_token_id: TokenIdentifier,
        locked_asset_factory_address: Address,
    ) -> SCResult<()> {
        require!(
            asset_token_id.is_valid_esdt_identifier(),
            "Asset token ID is not a valid esdt identifier"
        );

        self.asset_token_id().set(&asset_token_id);
        self.locked_asset_factory_address()
            .set(&locked_asset_factory_address);
        Ok(())
    }

    #[endpoint(startGlobalOperation)]
    fn start_planning(&self) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.global_op_start()?;
        Ok(())
    }

    #[endpoint(endGlobalOperation)]
    fn end_planning(&self) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.global_op_stop()?;
        Ok(())
    }

    #[endpoint(setCommunityDistribution)]
    fn set_community_distribution(
        &self,
        total_amount: Self::BigUint,
        spread_epoch: u64,
    ) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.require_global_op_ongoing()?;
        require!(total_amount > 0, "Zero amount");
        require!(
            spread_epoch >= self.blockchain().get_block_epoch(),
            "Spread epoch in the past"
        );
        require!(
            self.community_distribution_list()
                .front()
                .map(|community_distrib| community_distrib.spread_epoch)
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
        Ok(())
    }

    #[endpoint(setPerUserDistributedLockedAssets)]
    fn set_per_user_distributed_locked_assets(
        &self,
        spread_epoch: u64,
        #[var_args] user_locked_assets: VarArgs<MultiArg2<Address, Self::BigUint>>,
    ) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.require_global_op_ongoing()?;
        self.require_community_distribution_list_not_empty()?;

        require!(!user_locked_assets.is_empty(), "Empty assets vec");
        self.add_all_user_assets_to_map(spread_epoch, user_locked_assets)
    }

    #[endpoint(claimLockedAssets)]
    fn claim_locked_assets(&self) -> SCResult<Self::BigUint> {
        self.require_global_op_not_ongoing()?;
        self.require_unlock_period_not_empty()?;
        self.require_community_distribution_list_not_empty()?;

        let caller = self.blockchain().get_caller();
        let mut cummulated_amount = 0u64.into();

        let locked_assets = self.calculate_user_locked_assets(&caller, true);
        if locked_assets.is_empty() {
            return Ok(cummulated_amount);
        }

        let to = self.locked_asset_factory_address().get();
        let gas_limit_per_execute =
            self.blockchain().get_gas_left() / (locked_assets.len() as u64 + 1);

        let unlock_period = self.unlock_period().get();
        for (amount, spread_epoch) in locked_assets.iter() {
            self.locked_asset_factory_proxy(to.clone())
                .create_and_forward_custom_period(
                    amount.clone(),
                    caller.clone(),
                    *spread_epoch,
                    unlock_period.clone(),
                )
                .with_gas_limit(gas_limit_per_execute)
                .execute_on_dest_context_ignore_result();
            cummulated_amount += amount;
        }

        Ok(cummulated_amount)
    }

    #[endpoint(clearUnclaimableAssets)]
    fn clear_unclaimable_assets(&self) -> SCResult<usize> {
        let biggest_unclaimable_asset_epoch = self.get_biggest_unclaimable_asset_epoch();
        self.undo_user_assets_between_epochs(0, biggest_unclaimable_asset_epoch)
    }

    #[endpoint(undoLastCommunityDistribution)]
    fn undo_last_community_distrib(&self) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.require_global_op_ongoing()?;
        self.require_community_distribution_list_not_empty()?;
        self.community_distribution_list().pop_front();
        Ok(())
    }

    #[endpoint(undoUserDistributedAssetsBetweenEpochs)]
    fn undo_user_assets_between_epochs(&self, lower: u64, higher: u64) -> SCResult<usize> {
        only_owner!(self, "Permission denied");
        self.require_global_op_ongoing()?;
        self.require_community_distribution_list_not_empty()?;
        require!(lower <= higher, "Bad input values");
        Ok(self.remove_asset_entries_between_epochs(lower, higher))
    }

    #[only_owner]
    #[endpoint(setUnlockPeriod)]
    fn set_unlock_period(
        &self,
        #[var_args] milestones: MultiArgVec<UnlockMilestone>,
    ) -> SCResult<()> {
        self.validate_unlock_milestones(&milestones)?;
        self.unlock_period()
            .set(&UnlockPeriod::from(milestones.into_vec()));
        Ok(())
    }

    #[view(calculateLockedAssets)]
    fn calculate_locked_assets_view(&self, address: Address) -> SCResult<Self::BigUint> {
        self.require_global_op_not_ongoing()?;
        self.require_community_distribution_list_not_empty()?;
        let locked_assets = self.calculate_user_locked_assets(&address, false);

        let mut cummulated_amount = 0u64.into();
        for (amount, _) in locked_assets.iter() {
            cummulated_amount += amount;
        }
        Ok(cummulated_amount)
    }

    #[view(getLastCommunityDistributionAmountAndEpoch)]
    fn get_last_community_distrib_amount_and_epoch(&self) -> MultiResult2<Self::BigUint, u64> {
        self.community_distribution_list()
            .front()
            .map(|last_community_distrib| {
                (
                    last_community_distrib.total_amount,
                    last_community_distrib.spread_epoch,
                )
            })
            .unwrap_or((0u64.into(), 0u64))
            .into()
    }

    fn validate_unlock_milestones(
        &self,
        unlock_milestones: &VarArgs<UnlockMilestone>,
    ) -> SCResult<()> {
        require!(!unlock_milestones.is_empty(), "Empty param");

        let mut percents_sum: u8 = 0;
        let mut last_milestone_unlock_epoch: u64 = 0;

        for milestone in unlock_milestones.0.clone() {
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
        Ok(())
    }

    fn add_all_user_assets_to_map(
        &self,
        spread_epoch: u64,
        user_assets: VarArgs<MultiArg2<Address, Self::BigUint>>,
    ) -> SCResult<()> {
        let mut last_community_distrib = self.community_distribution_list().front().unwrap();
        require!(
            spread_epoch == last_community_distrib.spread_epoch,
            "Bad spread epoch"
        );
        for user_asset_multiarg in user_assets.into_vec() {
            let (caller, asset_amount) = user_asset_multiarg.into_tuple();
            require!(asset_amount > 0, "Zero amount");
            require!(
                last_community_distrib.after_planning_amount >= asset_amount,
                "User assets sums above community total assets"
            );
            last_community_distrib.after_planning_amount -= asset_amount.clone();
            self.add_user_locked_asset_entry(caller, asset_amount, spread_epoch)?;
        }
        self.community_distribution_list().pop_front();
        self.community_distribution_list()
            .push_front(last_community_distrib);
        Ok(())
    }

    fn add_user_locked_asset_entry(
        &self,
        caller: Address,
        asset_amount: Self::BigUint,
        spread_epoch: u64,
    ) -> SCResult<()> {
        let key = UserLockedAssetKey {
            caller,
            spread_epoch,
        };
        require!(
            !self.user_locked_asset_map().contains_key(&key),
            "Vector has duplicates"
        );
        self.user_locked_asset_map().insert(key, asset_amount);
        Ok(())
    }

    fn calculate_user_locked_assets(
        &self,
        address: &Address,
        delete_after_visit: bool,
    ) -> Vec<(Self::BigUint, Epoch)> {
        let current_epoch = self.blockchain().get_block_epoch();
        let mut locked_assets = Vec::<(Self::BigUint, Epoch)>::new();

        for community_distrib in self
            .community_distribution_list()
            .iter()
            .take(MAX_CLAIMABLE_DISTRIBUTION_ROUNDS)
            .filter(|x| x.spread_epoch <= current_epoch)
        {
            let user_asset_key = UserLockedAssetKey {
                caller: address.clone(),
                spread_epoch: community_distrib.spread_epoch,
            };

            if let Some(asset_amount) = self.user_locked_asset_map().get(&user_asset_key) {
                locked_assets.push((asset_amount, user_asset_key.spread_epoch));

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
            .map(|community_distrib| community_distrib.spread_epoch)
            .unwrap_or_default()
    }

    fn remove_asset_entries_between_epochs(&self, lower: u64, higher: u64) -> usize {
        if higher == 0 {
            return 0;
        }

        if higher < lower {
            return 0;
        }

        let mut to_remove_keys = Vec::new();
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

            self.user_locked_asset_map().remove(key);
        }
        map_len_before - self.user_locked_asset_map().len()
    }

    fn require_community_distribution_list_not_empty(&self) -> SCResult<()> {
        require!(
            !self.community_distribution_list().is_empty(),
            "Empty community assets list"
        );
        Ok(())
    }

    fn require_unlock_period_not_empty(&self) -> SCResult<()> {
        require!(!self.unlock_period().is_empty(), "Empty unlock schedule");
        Ok(())
    }

    #[endpoint(deleteUserDistributedLockedAssets)]
    fn delete_user_distributed_locked_assets(
        &self,
        spread_epoch: u64,
        address: Address,
    ) -> SCResult<()> {
        self.require_global_op_ongoing()?;
        self.user_locked_asset_map().remove(&UserLockedAssetKey {
            caller: address,
            spread_epoch: spread_epoch,
        });
        Ok(())
    }

    #[view(getAllUsersDistributedLockedAssets)]
    fn get_all_users_distributed_locked_assets(
        &self,
        spread_epoch: u64,
    ) -> MultiResultVec<UserLockedAssetKey> {
        self.user_locked_asset_map()
            .iter()
            .filter(|x| x.0.spread_epoch == spread_epoch)
            .map(|x| x.0)
            .collect()
    }

    #[view(getUnlockPeriod)]
    #[storage_mapper("unlock_period")]
    fn unlock_period(&self) -> SingleValueMapper<Self::Storage, UnlockPeriod>;

    #[view(getCommunityDistributionList)]
    #[storage_mapper("community_distribution_list")]
    fn community_distribution_list(
        &self,
    ) -> LinkedListMapper<Self::Storage, CommunityDistribution<Self::BigUint>>;

    #[storage_mapper("user_locked_asset_map")]
    fn user_locked_asset_map(
        &self,
    ) -> SafeMapMapper<Self::Storage, UserLockedAssetKey, Self::BigUint>;

    #[storage_mapper("locked_asset_factory_address")]
    fn locked_asset_factory_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getAssetTokenId)]
    #[storage_mapper("asset_token_id")]
    fn asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;
}
