#![no_std]
#![feature(generic_associated_types)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::FarmTokenAttributes;
use config::State;

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct FarmTokenAttributesV1_2<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub original_entering_epoch: u64,
    pub entering_epoch: u64,
    pub apr_multiplier: u8,
    pub with_locked_rewards: bool,
    pub initial_farming_amount: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
}

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
        old_attrs: FarmTokenAttributesV1_2<Self::Api>,
        orig_caller: ManagedAddress,
    ) -> EsdtTokenPayment<Self::Api> {
        require!(self.state().get() == State::Active, "not active");

        require!(!self.farm_migration_config().is_empty(), "empty config");
        let config = self.farm_migration_config().get();
        require!(!config.migration_role.is_old(), "bad config");

        let caller = self.blockchain().get_caller();
        require!(caller == config.old_farm_address, "bad caller");

        //Make sure this is the right farm SC based on old farm token attrs.
        require!(
            old_attrs.with_locked_rewards == config.migration_role.is_new_with_lock(),
            "bad lock option"
        );

        let payments = self.call_value().all_esdt_transfers();
        let payments_len = payments.len();
        require!(payments_len == 1, "bad payments len");

        let farming_tokens = payments.get(0);
        require!(farming_tokens.amount != 0u64, "bad farming amount");
        require!(
            farming_tokens.token_identifier == self.farming_token_id().get(),
            "bad farming token id"
        );

        let new_pos_token_id = self.farm_token_id().get();
        let new_pos_amount = farming_tokens.amount.clone();

        //Note that this function does not modify the farm supply
        let new_pos_nonce = self.nft_create_tokens(
            &new_pos_token_id,
            &new_pos_amount,
            &FarmTokenAttributes {
                reward_per_share: old_attrs.reward_per_share,
                entering_epoch: old_attrs.entering_epoch,
                original_entering_epoch: old_attrs.original_entering_epoch,
                initial_farming_amount: farming_tokens.amount,
                compounded_reward: BigUint::zero(),
                current_farm_amount: new_pos_amount.clone(),
            },
        );

        // Use this function since it works regardless of wasm ocasional unalignment.
        self.transfer_execute_custom(
            &orig_caller,
            &new_pos_token_id,
            new_pos_nonce,
            &new_pos_amount,
            &OptionalValue::None,
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

        self.farm_migration_config().set(&FarmMigrationConfig {
            migration_role,
            old_farm_address,
            old_farm_token_id,
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

    #[only_owner]
    #[endpoint(setFarmTokenSupply)]
    fn set_farm_token_supply(&self, supply: BigUint) {
        self.farm_token_supply().set(&supply);
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

    pub fn is_new_with_lock(&self) -> bool {
        self == &FarmMigrationRole::NewWithLock
    }
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct FarmMigrationConfig<M: ManagedTypeApi> {
    migration_role: FarmMigrationRole,
    old_farm_address: ManagedAddress<M>,
    old_farm_token_id: TokenIdentifier<M>,
}
