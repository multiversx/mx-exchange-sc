#![no_std]

use locked_asset_token::StakingEntry;

elrond_wasm::imports!();

pub mod locked_asset_token;

pub type SnapshotEntry<M> = MultiValue2<ManagedAddress<M>, BigUint<M>>;
pub const UNBOND_EPOCHS: u64 = 10;

#[elrond_wasm::contract]
pub trait MetabondingStaking: locked_asset_token::LockedAssetTokenModule {
    #[init]
    fn init(
        &self,
        locked_asset_token_id: TokenIdentifier,
        locked_asset_factory_address: ManagedAddress,
    ) {
        self.locked_asset_token_id().set(&locked_asset_token_id);
        self.locked_asset_factory_address()
            .set(&locked_asset_factory_address);
    }

    #[payable("*")]
    #[endpoint(stakeLockedAsset)]
    fn stake_locked_asset(&self) {
        let payments = self.call_value().all_esdt_transfers();
        self.require_all_locked_asset_payments(&payments);

        let caller = self.blockchain().get_caller();
        let new_locked_asset_token = self.merge_locked_asset_tokens_if_needed(&caller, payments);

        self.total_locked_asset_supply()
            .update(|total_supply| *total_supply += &new_locked_asset_token.amount);

        self.staking_entry_for_user(&caller).set(&StakingEntry::new(
            new_locked_asset_token.token_nonce,
            new_locked_asset_token.amount,
        ));
        self.user_list().insert(caller);
    }

    #[endpoint]
    fn unstake(&self) {
        let caller = self.blockchain().get_caller();
        let entry_mapper = self.staking_entry_for_user(&caller);
        require!(!entry_mapper.is_empty(), "Must stake first");

        let mut staking_entry: StakingEntry<Self::Api> = entry_mapper.get();
        require!(!staking_entry.is_unstaked(), "Already unstaked");

        let current_epoch = self.blockchain().get_block_epoch();
        staking_entry.opt_unbond_epoch = Some(current_epoch + UNBOND_EPOCHS);

        entry_mapper.set(&staking_entry);
    }

    #[endpoint]
    fn unbond(&self) {
        let caller = self.blockchain().get_caller();
        let entry_mapper = self.staking_entry_for_user(&caller);
        require!(!entry_mapper.is_empty(), "Must stake first");

        let staking_entry: StakingEntry<Self::Api> = entry_mapper.get();
        require!(staking_entry.is_unstaked(), "Must unstake first");

        let current_epoch = self.blockchain().get_block_epoch();
        let unbond_epoch = staking_entry.opt_unbond_epoch.unwrap();
        require!(current_epoch >= unbond_epoch, "Unbond period in progress");

        self.total_locked_asset_supply()
            .update(|total_supply| *total_supply -= &staking_entry.amount);

        entry_mapper.clear();
        self.user_list().swap_remove(&caller);

        let locked_asset_token_id = self.locked_asset_token_id().get();
        self.send().direct(
            &caller,
            &locked_asset_token_id,
            staking_entry.nonce,
            &staking_entry.amount,
            &[],
        );
    }

    #[view(getStakedAmountForUser)]
    fn get_staked_amount_for_user(&self, user_address: ManagedAddress) -> BigUint {
        let entry_mapper = self.staking_entry_for_user(&user_address);
        if entry_mapper.is_empty() {
            BigUint::zero()
        } else {
            let entry: StakingEntry<Self::Api> = entry_mapper.get();
            if entry.is_unstaked() {
                BigUint::zero()
            } else {
                entry.amount
            }
        }
    }

    #[view(getUserStakedPosition)]
    fn get_user_staked_position(&self, user_address: ManagedAddress) -> StakingEntry<Self::Api> {
        let entry_mapper = self.staking_entry_for_user(&user_address);

        if entry_mapper.is_empty() {
            StakingEntry::new(0u64, BigUint::zero())
        } else {
            entry_mapper.get()
        }
    }

    #[view(getSnapshot)]
    fn get_snapshot(&self) -> MultiValueEncoded<SnapshotEntry<Self::Api>> {
        let mut result = MultiValueEncoded::new();

        for user_address in self.user_list().iter() {
            let entry: StakingEntry<Self::Api> = self.staking_entry_for_user(&user_address).get();
            if !entry.is_unstaked() {
                result.push((user_address, entry.amount).into());
            }
        }

        result
    }
}
