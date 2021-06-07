#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use modules::*;

type Epoch = u64;

const GAS_CHECK_FREQUENCY: usize = 100;
const MAX_CLAIMABLE_DISTRIBUTION_ROUNDS: usize = 4;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct UserLockedAssetKey {
    pub user_address: Address,
    pub spread_epoch: u64,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct CommunityDistribution<BigUint: BigUintApi> {
    pub total_amount: BigUint,
    pub spread_epoch: u64,
    pub after_planning_amount: BigUint,
}

#[elrond_wasm_derive::contract]
pub trait Distribution: asset::AssetModule + global_op::GlobalOperationModule {
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
    fn set_community_distribution(
        &self,
        total_amount: Self::BigUint,
        spread_epoch: u64,
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
        self.require_community_distribution_list_not_empty()?;

        let caller = self.blockchain().get_caller();
        let locked_assets = self.calculate_user_locked_assets(&caller, true);
        let to = self.locked_asset_factory_address().get();
        let gas_limit_per_execute =
            self.blockchain().get_gas_left() / (locked_assets.len() as u64 + 1);

        let mut cummulated_amount = Self::BigUint::zero();
        for (amount, spread_epoch) in locked_assets.iter() {
            self.locked_asset_factory_proxy(to.clone())
                .create_and_forward(
                    amount.clone(),
                    caller.clone(),
                    *spread_epoch,
                    OptionalArg::None,
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

    #[view(calculateLockedAssets)]
    fn calculate_locked_assets_view(&self, address: Address) -> SCResult<Self::BigUint> {
        self.require_global_op_not_ongoing()?;
        self.require_community_distribution_list_not_empty()?;
        let locked_assets = self.calculate_user_locked_assets(&address, false);

        let mut cummulated_amount = Self::BigUint::zero();
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
            .unwrap_or((Self::BigUint::zero(), 0u64))
            .into()
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
            let (user_address, asset_amount) = user_asset_multiarg.into_tuple();
            require!(
                last_community_distrib.after_planning_amount >= asset_amount,
                "User assets sums above community total assets"
            );
            last_community_distrib.after_planning_amount -= asset_amount.clone();
            self.add_user_locked_asset_entry(user_address, asset_amount, spread_epoch)?;
        }
        self.community_distribution_list().pop_front();
        self.community_distribution_list()
            .push_front(last_community_distrib);
        Ok(())
    }

    fn add_user_locked_asset_entry(
        &self,
        user_address: Address,
        asset_amount: Self::BigUint,
        spread_epoch: u64,
    ) -> SCResult<()> {
        let key = UserLockedAssetKey {
            user_address,
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
                user_address: address.clone(),
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
        for (user_asset_index, user_asset_key) in self.user_locked_asset_map().keys().enumerate() {
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
            self.user_locked_asset_map().remove(key);
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

    #[storage_mapper("community_distribution_list")]
    fn community_distribution_list(
        &self,
    ) -> LinkedListMapper<Self::Storage, CommunityDistribution<Self::BigUint>>;

    #[storage_mapper("user_locked_asset_map")]
    fn user_locked_asset_map(&self) -> MapMapper<Self::Storage, UserLockedAssetKey, Self::BigUint>;

    #[storage_mapper("locked_asset_factory_address")]
    fn locked_asset_factory_address(&self) -> SingleValueMapper<Self::Storage, Address>;
}
