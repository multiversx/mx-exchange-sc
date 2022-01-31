elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{FarmTokenAttributes, FarmTokenAttributesV1_2};
use config::State;

#[elrond_wasm::module]
pub trait MigrationModule:
    config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + rewards::RewardsModule
{
    #[payable("*")]
    #[endpoint(migrateFromV1_2Farm)]
    fn migrate_from_v1_2_farm(
        &self,
        #[var_args] orig_caller_opt: OptionalArg<ManagedAddress>,
    ) -> EsdtTokenPayment<Self::Api> {
        require!(self.farm_migration_config().is_empty(), "empty config");
        let config = self.farm_migration_config().get();
        require!(!config.migration_role.is_old(), "bad config");

        let caller = self.blockchain().get_caller();
        require!(caller == config.old_farm_address, "bad caller");

        let payments = self.call_value().all_esdt_transfers();
        require!(payments.len() == 3, "bad payments len");

        let old_position = payments.get(0);
        require!(old_position.amount != 0u64, "bad farm amount");
        require!(
            old_position.token_identifier == config.old_farm_token_id,
            "bad farm token id"
        );

        let farming_tokens = payments.get(1);
        require!(farming_tokens.amount != 0u64, "bad farming amount");
        require!(
            farming_tokens.token_identifier == self.farming_token_id().get(),
            "bad farming token id"
        );

        let reward = payments.get(2);
        require!(reward.amount != 0u64, "bad reward amount");
        let reward_token_id = self.reward_token_id().get();
        require!(
            reward.token_identifier == reward_token_id,
            "bad reward token id"
        );

        // The actual work starts here
        self.reward_reserve().update(|x| *x += &reward.amount);

        let old_attrs: FarmTokenAttributesV1_2<Self::Api> = self
            .blockchain()
            .get_esdt_token_data(
                &self.blockchain().get_sc_address(),
                &old_position.token_identifier,
                old_position.token_nonce,
            )
            .decode_attributes()
            .unwrap();
        require!(
            old_attrs.with_locked_rewards == config.migration_role.is_new_with_lock(),
            "bad lock option"
        );

        // Do not call burn_farm_tokens since this farm tokens belong to other contract
        // which already updated its farm token supply counter.
        self.send().esdt_local_burn(
            &old_position.token_identifier,
            old_position.token_nonce,
            &old_position.amount,
        );

        let new_pos_token_id = self.farm_token_id().get();
        let new_pos_amount = old_position.amount;

        // Use this function because it also updates the farm token supply for this contract instance.
        let new_pos_nonce = self.mint_farm_tokens(
            &new_pos_token_id,
            &new_pos_amount,
            &FarmTokenAttributes {
                reward_per_share: old_attrs.reward_per_share,
                entering_epoch: old_attrs.entering_epoch,
                original_entering_epoch: old_attrs.original_entering_epoch,
                initial_farming_amount: old_attrs.initial_farming_amount,
                compounded_reward: old_attrs.compounded_reward,
                current_farm_amount: old_attrs.current_farm_amount,
            },
        );

        let orig_caller = orig_caller_opt
            .into_option()
            .unwrap_or_else(|| caller.clone());

        // Use this function since it works regardless of wasm ocasional unalignment.
        self.transfer_execute_custom(
            &orig_caller,
            &new_pos_token_id,
            new_pos_nonce,
            &new_pos_amount,
            &OptionalArg::None,
        );

        EsdtTokenPayment::new(new_pos_token_id, new_pos_nonce, new_pos_amount)
    }

    // Each farm that will be migrated and the newer version to which we migrate to
    // will have to be configured using this function.
    #[only_owner]
    #[endpoint(setFarmMigrationConfig)]
    fn set_farm_migration_config(
        &self,
        old_farm_address: ManagedAddress,
        old_farm_token_id: TokenIdentifier,
        new_farm_address: ManagedAddress,
        new_farm_with_lock_address: ManagedAddress,
    ) {
        let sc_address = self.blockchain().get_sc_address();
        let migration_role = if sc_address == old_farm_address {
            FarmMigrationRole::Old
        } else if sc_address == new_farm_address {
            FarmMigrationRole::New
        } else if sc_address == new_farm_with_lock_address {
            FarmMigrationRole::NewWithLock
        } else {
            sc_panic!("bad config")
        };
        require!(migration_role.is_new(), "bad config");

        self.farm_migration_config().set(&FarmMigrationConfig {
            migration_role,
            old_farm_address,
            old_farm_token_id,
            new_farm_address,
            new_farm_with_lock_address,
        });
    }

    #[endpoint(setRpsAndStartRewards)]
    fn set_rps_and_start_rewards(&self, rps: BigUint) {
        require!(
            !self.produce_rewards_enabled().get(),
            "rewards already enabled"
        );

        require!(!self.farm_migration_config().is_empty(), "empty config");
        let config = self.farm_migration_config().get();
        require!(!config.migration_role.is_old(), "bad config");
        let caller = self.blockchain().get_caller();
        require!(caller == config.old_farm_address, "bad caller");

        self.reward_per_share().set(&rps);
        self.start_produce_rewards();
        self.state().set(&State::Active);
    }

    #[view(getFarmMigrationConfiguration)]
    #[storage_mapper("farm_migration_config")]
    fn farm_migration_config(&self) -> SingleValueMapper<FarmMigrationConfig<Self::Api>>;
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq)]
pub enum FarmMigrationRole {
    Old,
    New,
    NewWithLock,
}

impl FarmMigrationRole {
    pub fn is_old(&self) -> bool {
        self == &FarmMigrationRole::Old
    }
    pub fn is_new(&self) -> bool {
        self == &FarmMigrationRole::New
    }
    pub fn is_new_with_lock(&self) -> bool {
        self == &FarmMigrationRole::NewWithLock
    }
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct FarmMigrationConfig<M: ManagedTypeApi> {
    migration_role: FarmMigrationRole,
    old_farm_address: ManagedAddress<M>,
    old_farm_token_id: TokenIdentifier<M>,
    new_farm_address: ManagedAddress<M>,
    new_farm_with_lock_address: ManagedAddress<M>,
}
