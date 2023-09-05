#![no_std]

multiversx_sc::imports!();

pub mod events;
pub mod locked_asset_token;

use locked_asset_token::UserEntry;

pub type SnapshotEntry<M> = MultiValue2<ManagedAddress<M>, BigUint<M>>;
pub const UNBOND_EPOCHS: u64 = 3;

#[multiversx_sc::contract]
pub trait MetabondingStaking:
    locked_asset_token::LockedAssetTokenModule + events::EventsModule
{
    #[init]
    fn init(
        &self,
        locked_asset_token_id: TokenIdentifier,
        locked_asset_factory_address: ManagedAddress,
    ) {
        self.locked_asset_token_id()
            .set_if_empty(&locked_asset_token_id);
        self.locked_asset_factory_address()
            .set_if_empty(&locked_asset_factory_address);
    }

    #[payable("*")]
    #[endpoint(stakeLockedAsset)]
    fn stake_locked_asset(&self) {
        let payments = self.call_value().all_esdt_transfers().clone_value();
        self.require_all_locked_asset_payments(&payments);

        let caller = self.blockchain().get_caller();
        let entry_mapper = self.entry_for_user(&caller);
        let new_entry = self.create_new_entry_by_merging_tokens(&entry_mapper, payments);

        self.total_locked_asset_supply()
            .update(|total_supply| *total_supply += new_entry.get_total_amount());

        self.stake_event(&caller, &new_entry);

        entry_mapper.set(&new_entry);
        let _ = self.user_list().insert(caller);
    }

    #[endpoint]
    fn unstake(&self, amount: BigUint) {
        let caller = self.blockchain().get_caller();
        let entry_mapper = self.entry_for_user(&caller);
        require!(!entry_mapper.is_empty(), "Must stake first");

        let mut user_entry: UserEntry<Self::Api> = entry_mapper.get();
        require!(
            amount <= user_entry.stake_amount,
            "Trying to unstake too much"
        );

        let current_epoch = self.blockchain().get_block_epoch();
        user_entry.unbond_epoch = current_epoch + UNBOND_EPOCHS;
        user_entry.stake_amount -= &amount;
        user_entry.unstake_amount += amount;

        self.unstake_event(&caller, &user_entry);

        entry_mapper.set(&user_entry);
    }

    #[endpoint]
    fn unbond(&self) {
        let caller = self.blockchain().get_caller();
        let entry_mapper = self.entry_for_user(&caller);
        require!(!entry_mapper.is_empty(), "Must stake first");

        let mut user_entry: UserEntry<Self::Api> = entry_mapper.get();
        let unstake_amount = user_entry.unstake_amount.clone();
        require!(unstake_amount > 0, "Must unstake first");

        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            current_epoch >= user_entry.unbond_epoch,
            "Unbond period in progress"
        );

        self.total_locked_asset_supply()
            .update(|total_supply| *total_supply -= &unstake_amount);

        let opt_entry_after_action = if user_entry.stake_amount == 0 {
            entry_mapper.clear();
            self.user_list().swap_remove(&caller);

            None
        } else {
            user_entry.unstake_amount = BigUint::zero();
            user_entry.unbond_epoch = u64::MAX;
            entry_mapper.set(&user_entry);

            Some(&user_entry)
        };

        let locked_asset_token_id = self.locked_asset_token_id().get();
        self.send().direct_esdt(
            &caller,
            &locked_asset_token_id,
            user_entry.token_nonce,
            &unstake_amount,
        );

        self.unbond_event(&caller, opt_entry_after_action);
    }

    #[view(getStakedAmountForUser)]
    fn get_staked_amount_for_user(&self, user_address: ManagedAddress) -> BigUint {
        let entry_mapper = self.entry_for_user(&user_address);
        if entry_mapper.is_empty() {
            BigUint::zero()
        } else {
            let entry: UserEntry<Self::Api> = entry_mapper.get();

            entry.stake_amount
        }
    }

    #[view(getUserEntry)]
    fn get_user_entry(&self, user_address: ManagedAddress) -> OptionalValue<UserEntry<Self::Api>> {
        let entry_mapper = self.entry_for_user(&user_address);

        if !entry_mapper.is_empty() {
            OptionalValue::Some(entry_mapper.get())
        } else {
            OptionalValue::None
        }
    }

    #[view(getSnapshot)]
    fn get_snapshot(&self) -> MultiValueEncoded<SnapshotEntry<Self::Api>> {
        let mut result = MultiValueEncoded::new();

        for user_address in self.user_list().iter() {
            let entry: UserEntry<Self::Api> = self.entry_for_user(&user_address).get();
            if entry.stake_amount > 0 {
                result.push((user_address, entry.stake_amount).into());
            }
        }

        result
    }
}
