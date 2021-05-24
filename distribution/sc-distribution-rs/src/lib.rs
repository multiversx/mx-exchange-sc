#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use distrib_common::*;
use modules::*;

const GAS_CHECK_FREQUENCY: usize = 100;
const MAX_CLAIMABLE_DISTRIBUTION_ROUNDS: usize = 4;

#[elrond_wasm_derive::contract]
pub trait EsdtDistribution: asset::AssetModule + global_op::GlobalOperationModule {
    #[proxy]
    fn locked_asset_factory_proxy(
        &self,
        to: Address,
    ) -> sc_locked_asset_factory::Proxy<Self::SendApi>;

    #[init]
    fn init(&self, asset_token_id: TokenIdentifier, locked_asset_factory_address: Address) {
        self.asset_token_id().set(&asset_token_id);
        self.locked_asset_factory_address()
            .set(&locked_asset_factory_address);
    }

    #[endpoint(startGlobalOperation)]
    fn start_planning(&self) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.global_op_start();
        Ok(())
    }

    #[endpoint(endGlobalOperation)]
    fn end_planning(&self) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.global_op_stop();
        Ok(())
    }

    #[endpoint(setCommunityDistribution)]
    fn set_community_distrib(
        &self,
        total_amount: Self::BigUint,
        spread_epoch: u64,
        #[var_args] unlock_milestones: VarArgs<UnlockMilestone>,
    ) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.require_global_op_ongoing()?;
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
        self.validate_unlock_milestones(&unlock_milestones)?;
        let distrib = CommunityDistribution {
            total_amount: total_amount.clone(),
            spread_epoch,
            after_planning_amount: total_amount,
            unlock_milestones: unlock_milestones.into_vec(),
        };
        self.community_distribution_list().push_front(distrib);
        Ok(())
    }

    //Currently duplicated function.
    fn validate_unlock_milestones(
        &self,
        unlock_milestones: &VarArgs<UnlockMilestone>,
    ) -> SCResult<()> {
        let mut percents_sum: u8 = 0;
        let mut last_milestone_unlock_epoch: u64 = 0;
        for milestone in unlock_milestones.0.clone() {
            require!(
                milestone.unlock_epoch > last_milestone_unlock_epoch,
                "Unlock epochs not in order"
            );
            require!(
                milestone.unlock_percent <= 100,
                "Unlock percent more than 100"
            );
            last_milestone_unlock_epoch = milestone.unlock_epoch;
            percents_sum += milestone.unlock_percent;
        }
        if !unlock_milestones.is_empty() {
            require!(percents_sum == 100, "Percents do not sum up to 100");
        }
        Ok(())
    }

    #[endpoint(setPerUserDistributedAssets)]
    fn set_per_user_distributed_assets(
        &self,
        spread_epoch: u64,
        #[var_args] user_assets: VarArgs<MultiArg2<Address, Self::BigUint>>,
    ) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.require_global_op_ongoing()?;
        self.require_community_distribution_list_not_empty()?;
        require!(!user_assets.is_empty(), "Empty assets vec");
        self.add_all_user_assets_to_map(spread_epoch, user_assets, false)
    }

    #[endpoint(setPerUserDistributedLockedAssets)]
    fn set_per_user_distributed_locked_assets(
        &self,
        spread_epoch: u64,
        #[var_args] user_assets: VarArgs<MultiArg2<Address, Self::BigUint>>,
    ) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.require_global_op_ongoing()?;
        self.require_community_distribution_list_not_empty()?;
        require!(
            !self
                .community_distribution_list()
                .front()
                .unwrap()
                .unlock_milestones
                .is_empty(),
            "No unlock milestones set"
        );
        require!(!user_assets.is_empty(), "Empty assets vec");
        self.add_all_user_assets_to_map(spread_epoch, user_assets, true)
    }

    #[endpoint(claimAssets)]
    fn claim_assets(&self) -> SCResult<Self::BigUint> {
        self.require_global_op_not_ongoing()?;
        self.require_community_distribution_list_not_empty()?;
        let caller = self.blockchain().get_caller();
        let (assets_amounts, _) = self.calculate_user_assets(&caller, false, true);
        let cummulated_amount = self.sum_of(&assets_amounts);
        self.mint_and_send_assets(&caller, &cummulated_amount);
        Ok(cummulated_amount)
    }

    #[endpoint(claimLockedAssets)]
    fn claim_locked_assets(&self) -> SCResult<Self::BigUint> {
        self.require_global_op_not_ongoing()?;
        self.require_community_distribution_list_not_empty()?;
        let caller = self.blockchain().get_caller();
        let (assets_amounts, unlock_milestones_vec) =
            self.calculate_user_assets(&caller, true, true);
        let to = self.locked_asset_factory_address().get();
        let gas_limit_per_execute =
            self.blockchain().get_gas_left() / (assets_amounts.len() as u64 + 1);
        for it in assets_amounts.iter().zip(unlock_milestones_vec) {
            let (amount, unlock_milestones) = it;
            self.locked_asset_factory_proxy(to.clone())
                .create_and_forward_custom_schedule(
                    amount.clone(),
                    caller.clone(),
                    MultiArgVec::from(unlock_milestones),
                )
                .execute_on_dest_context(gas_limit_per_execute);
        }

        let cummulated_amount = self.sum_of(&assets_amounts);
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

    #[view(calculateAssets)]
    fn calculate_assets_view(&self, address: Address) -> SCResult<Self::BigUint> {
        self.require_global_op_not_ongoing()?;
        self.require_community_distribution_list_not_empty()?;
        let (assets_amounts, _) = self.calculate_user_assets(&address, false, false);
        let cummulated_amount = self.sum_of(&assets_amounts);
        Ok(cummulated_amount)
    }

    #[view(calculateLockedAssets)]
    fn calculate_locked_assets_view(&self, address: Address) -> SCResult<Self::BigUint> {
        self.require_global_op_not_ongoing()?;
        self.require_community_distribution_list_not_empty()?;
        let (assets_amounts, _) = self.calculate_user_assets(&address, true, false);
        let cummulated_amount = self.sum_of(&assets_amounts);
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
            .unwrap_or((Self::BigUint::zero(), 0u64))
            .into()
    }

    #[view(getLastCommunityDistributionUnlockMilestones)]
    fn get_last_community_distrib_unlock_milestones(&self) -> MultiResultVec<UnlockMilestone> {
        self.community_distribution_list()
            .front()
            .map(|last_community_distrib| last_community_distrib.unlock_milestones)
            .unwrap_or_default()
            .into()
    }

    fn add_all_user_assets_to_map(
        &self,
        spread_epoch: u64,
        user_assets: VarArgs<MultiArg2<Address, Self::BigUint>>,
        locked_assets: bool,
    ) -> SCResult<()> {
        let mut last_community_distrib = self.community_distribution_list().front().unwrap();
        require!(
            spread_epoch == last_community_distrib.spread_epoch,
            "Bad spread epoch"
        );
        for user_asset_multiarg in user_assets.into_vec() {
            let (user_address, asset_amount) = user_asset_multiarg.into_tuple();
            require!(
                last_community_distrib.after_planning_amount >= asset_amount,
                "User assets sums above community total assets"
            );
            last_community_distrib.after_planning_amount -= asset_amount.clone();
            self.add_user_asset_entry(user_address, asset_amount, spread_epoch, locked_assets)?;
        }
        self.community_distribution_list().pop_front();
        self.community_distribution_list()
            .push_front(last_community_distrib);
        Ok(())
    }

    fn add_user_asset_entry(
        &self,
        user_address: Address,
        asset_amount: Self::BigUint,
        spread_epoch: u64,
        locked_asset: bool,
    ) -> SCResult<()> {
        let user_asset_key = UserAssetKey {
            user_address,
            spread_epoch,
            locked_asset,
        };
        require!(
            !self.user_asset_map().contains_key(&user_asset_key),
            "Vector has duplicates"
        );
        self.user_asset_map().insert(user_asset_key, asset_amount);
        Ok(())
    }

    fn calculate_user_assets(
        &self,
        address: &Address,
        locked_asset: bool,
        delete_after_visit: bool,
    ) -> (Vec<Self::BigUint>, Vec<Vec<UnlockMilestone>>) {
        let current_epoch = self.blockchain().get_block_epoch();
        let mut amounts = Vec::<Self::BigUint>::new();
        let mut milestones = Vec::<Vec<UnlockMilestone>>::new();

        for community_distrib in self
            .community_distribution_list()
            .iter()
            .take(MAX_CLAIMABLE_DISTRIBUTION_ROUNDS)
            .filter(|x| x.spread_epoch <= current_epoch)
        {
            let user_asset_key = UserAssetKey {
                user_address: address.clone(),
                spread_epoch: community_distrib.spread_epoch,
                locked_asset,
            };

            if let Some(asset_amount) = self.user_asset_map().get(&user_asset_key) {
                amounts.push(asset_amount);
                milestones.push(community_distrib.unlock_milestones);

                if delete_after_visit {
                    self.user_asset_map().remove(&user_asset_key);
                }
            }
        }
        (amounts, milestones)
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
        for (user_asset_index, user_asset_key) in self.user_asset_map().keys().enumerate() {
            if (user_asset_index + 1) % GAS_CHECK_FREQUENCY == 0
                && self.blockchain().get_gas_left() < search_gas_limit
            {
                break;
            }
            if lower <= user_asset_key.spread_epoch && user_asset_key.spread_epoch <= higher {
                to_remove_keys.push(user_asset_key);
            }
        }

        for key in to_remove_keys.iter() {
            self.user_asset_map().remove(&key);
        }
        to_remove_keys.len()
    }

    fn require_community_distribution_list_not_empty(&self) -> SCResult<()> {
        require!(
            !self.community_distribution_list().is_empty(),
            "Empty community assets list"
        );
        Ok(())
    }

    fn sum_of(&self, vect: &[Self::BigUint]) -> Self::BigUint {
        let mut sum = Self::BigUint::zero();
        for item in vect.iter() {
            sum += item;
        }
        sum
    }

    #[storage_mapper("community_distribution_list")]
    fn community_distribution_list(
        &self,
    ) -> LinkedListMapper<Self::Storage, CommunityDistribution<Self::BigUint>>;

    #[storage_mapper("user_asset_map")]
    fn user_asset_map(&self) -> MapMapper<Self::Storage, UserAssetKey, Self::BigUint>;

    #[storage_mapper("locked_asset_factory_address")]
    fn locked_asset_factory_address(&self) -> SingleValueMapper<Self::Storage, Address>;
}
