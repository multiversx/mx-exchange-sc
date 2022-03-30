elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use config::State;

use super::config;
use super::farm_token;
use super::rewards;

mod farm_v1_4_contract_proxy {
    use common_structs::FarmTokenAttributes;

    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait Farm {
        #[payable("*")]
        #[endpoint(migrateFromV1_2Farm)]
        fn migrate_from_v1_2_farm(
            &self,
            farm_attributes: FarmTokenAttributes<Self::Api>,
            orig_caller: ManagedAddress,
        ) -> EsdtTokenPayment<Self::Api>;

        #[endpoint(setRpsAndStartRewards)]
        fn set_rps_and_start_rewards(&self, rps: BigUint);
    }
}

#[elrond_wasm::module]
pub trait MigrationModule:
    token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + config::ConfigModule
    + token_merge::TokenMergeModule
    + rewards::RewardsModule
{
    #[payable("*")]
    #[endpoint(migrateToNewFarmWithNoRewards)]
    fn migrate_to_new_farm_with_no_rewards(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_nonce] token_nonce: u64,
        #[payment_amount] amount: BigUint,
        orig_caller: ManagedAddress,
    ) -> SCResult<MultiResult2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>>> {
        self.migrate_to_new_farm(payment_token_id, token_nonce, amount, orig_caller, false)
    }

    #[payable("*")]
    #[endpoint(migrateToNewFarm)]
    fn migrate_to_new_farm_with_rewards(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_nonce] token_nonce: u64,
        #[payment_amount] amount: BigUint,
        orig_caller: ManagedAddress,
    ) -> SCResult<MultiResult2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>>> {
        self.migrate_to_new_farm(payment_token_id, token_nonce, amount, orig_caller, true)
    }

    fn migrate_to_new_farm(
        &self,
        payment_token_id: TokenIdentifier,
        token_nonce: u64,
        amount: BigUint,
        orig_caller: ManagedAddress,
        with_rewards: bool,
    ) -> SCResult<MultiResult2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>>> {
        require!(self.state().get() == State::Migrate, "bad state");
        require!(!self.farm_token_id().is_empty(), "No farm token");

        require!(!self.farm_migration_config().is_empty(), "empty config");
        let migration_config = self.farm_migration_config().get();
        require!(migration_config.migration_role.is_old(), "bad config");

        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Bad input token");
        require!(amount > 0u64, "Payment amount cannot be zero");

        let mut farm_attributes = self.get_farm_attributes(&payment_token_id, token_nonce)?;
        let mut reward_token_id = self.reward_token_id().get();

        let mut reward = if with_rewards {
            self.calculate_reward(
                &amount,
                &self.reward_per_share().get(),
                &farm_attributes.reward_per_share,
            )
        } else {
            BigUint::zero()
        };

        if reward > 0u64 {
            self.decrease_reward_reserve(&reward)?;
        }

        let farming_token_id = self.farming_token_id().get();
        let initial_farming_token_amount = self.rule_of_three_non_zero_result(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.initial_farming_amount,
        )?;
        reward += self.rule_of_three(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        );

        self.burn_farm_tokens(&payment_token_id, token_nonce, &amount);
        self.farming_token_reserve()
            .update(|x| *x -= &initial_farming_token_amount);

        let new_farm_dest = if farm_attributes.with_locked_rewards {
            migration_config.new_farm_with_lock_address
        } else {
            migration_config.new_farm_address
        };

        //Reset since the rewards will be send in this tx
        farm_attributes.reward_per_share = self.reward_per_share().get();

        let new_position = self
            .farm_v1_4_contract_proxy(new_farm_dest)
            .migrate_from_v1_2_farm(farm_attributes.clone(), orig_caller.clone())
            .add_token_transfer(farming_token_id, 0, initial_farming_token_amount)
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after));

        let mut reward_nonce = 0u64;
        self.send_rewards(
            &mut reward_token_id,
            &mut reward_nonce,
            &mut reward,
            &orig_caller,
            farm_attributes.with_locked_rewards,
            farm_attributes.original_entering_epoch,
            &OptionalArg::None,
        )?;

        Ok(MultiResult2::from((
            new_position,
            EsdtTokenPayment::new(reward_token_id, reward_nonce, reward),
        )))
    }

    #[only_owner]
    #[endpoint(setFarmMigrationConfig)]
    fn set_farm_migration_config(
        &self,
        old_farm_address: ManagedAddress,
        old_farm_token_id: TokenIdentifier,
        new_farm_address: ManagedAddress,
        new_farm_with_lock_address: ManagedAddress,
    ) -> SCResult<()> {
        let sc_address = self.blockchain().get_sc_address();
        require!(sc_address == old_farm_address, "bad config");

        self.farm_migration_config().set(&FarmMigrationConfig {
            migration_role: FarmMigrationRole::Old,
            old_farm_address,
            old_farm_token_id,
            new_farm_address,
            new_farm_with_lock_address,
        });
        Ok(())
    }

    // We also need to get the rps and transfer it to the new SC.
    #[only_owner]
    #[endpoint(stopRewardsAndMigrateRps)]
    fn stop_rewards_and_migrate_rps(&self) -> SCResult<()> {
        require!(!self.farm_migration_config().is_empty(), "empty config");
        let config = self.farm_migration_config().get();
        require!(config.migration_role.is_old(), "bad config");

        self.state().set(&State::Migrate);
        self.end_produce_rewards()?;

        let rps = self.reward_per_share().get();
        self.farm_v1_4_contract_proxy(config.new_farm_address)
            .set_rps_and_start_rewards(rps.clone())
            .execute_on_dest_context_ignore_result();

        self.farm_v1_4_contract_proxy(config.new_farm_with_lock_address)
            .set_rps_and_start_rewards(rps)
            .execute_on_dest_context_ignore_result();

        Ok(())
    }

    #[view(getFarmMigrationConfiguration)]
    #[storage_mapper("farm_migration_config")]
    fn farm_migration_config(&self) -> SingleValueMapper<FarmMigrationConfig<Self::Api>>;

    #[proxy]
    fn farm_v1_4_contract_proxy(
        &self,
        to: ManagedAddress,
    ) -> farm_v1_4_contract_proxy::Proxy<Self::Api>;
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq)]
pub enum FarmMigrationRole {
    Old,
    New,
    NewWithLock,
}

#[allow(dead_code)]
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
