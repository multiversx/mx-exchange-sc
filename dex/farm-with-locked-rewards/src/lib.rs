#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::FarmTokenAttributes;
use contexts::storage_cache::StorageCache;
use core::marker::PhantomData;
use fixed_supply_token::FixedSupplyToken;

use farm::{
    base_functions::{BaseFunctionsModule, ClaimRewardsResultType, DoubleMultiPayment, Wrapper},
    exit_penalty::{
        DEFAULT_BURN_GAS_LIMIT, DEFAULT_MINUMUM_FARMING_EPOCHS, DEFAULT_PENALTY_PERCENT,
    },
    EnterFarmResultType, ExitFarmWithPartialPosResultType, MAX_PERCENT,
};
use farm_base_impl::base_traits_impl::FarmContract;

#[multiversx_sc::contract]
pub trait Farm:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + locking_module::lock_with_energy_module::LockWithEnergyModule
    + farm_token::FarmTokenModule
    + utils::UtilsModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + sc_whitelist_module::SCWhitelistModule
    + events::EventsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm::base_functions::BaseFunctionsModule
    + farm::exit_penalty::ExitPenaltyModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + energy_query::EnergyQueryModule
{
    #[init]
    fn init(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        pair_contract_address: ManagedAddress,
        owner: ManagedAddress,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        self.base_farm_init(
            reward_token_id,
            farming_token_id,
            division_safety_constant,
            owner,
            admins,
        );

        self.penalty_percent().set_if_empty(DEFAULT_PENALTY_PERCENT);
        self.minimum_farming_epochs()
            .set_if_empty(DEFAULT_MINUMUM_FARMING_EPOCHS);
        self.burn_gas_limit().set_if_empty(DEFAULT_BURN_GAS_LIMIT);
        self.pair_contract_address().set(&pair_contract_address);

        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set_if_empty(current_epoch);

        // Farm position migration code
        let farm_token_mapper = self.farm_token();
        self.try_set_farm_position_migration_nonce(farm_token_mapper);
    }

    #[upgrade]
    fn upgrade(&self) {
        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set_if_empty(current_epoch);

        // Farm position migration code
        let farm_token_mapper = self.farm_token();
        self.try_set_farm_position_migration_nonce(farm_token_mapper);
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> EnterFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);

        self.migrate_old_farm_positions(&orig_caller);
        let boosted_rewards = self.claim_only_boosted_payment(&orig_caller);
        let boosted_rewards_payment = self.send_to_lock_contract_non_zero(
            self.reward_token_id().get(),
            boosted_rewards,
            caller.clone(),
            orig_caller.clone(),
        );

        let new_farm_token = self.enter_farm::<NoMintWrapper<Self>>(orig_caller.clone());
        self.send_payment_non_zero(&caller, &new_farm_token);

        self.update_energy_and_progress(&orig_caller);

        (new_farm_token, boosted_rewards_payment).into()
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> ClaimRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);

        self.migrate_old_farm_positions(&orig_caller);

        let payments = self.call_value().all_esdt_transfers().clone_value();
        let base_claim_rewards_result =
            self.claim_rewards_base::<NoMintWrapper<Self>>(orig_caller.clone(), payments);
        let output_farm_token_payment = base_claim_rewards_result.new_farm_token.payment.clone();
        self.send_payment_non_zero(&caller, &output_farm_token_payment);

        let rewards_payment = base_claim_rewards_result.rewards;
        let locked_rewards_payment = self.send_to_lock_contract_non_zero(
            rewards_payment.token_identifier,
            rewards_payment.amount,
            caller,
            orig_caller.clone(),
        );

        self.emit_claim_rewards_event::<_, FarmTokenAttributes<Self::Api>>(
            &orig_caller,
            base_claim_rewards_result.context,
            base_claim_rewards_result.new_farm_token,
            locked_rewards_payment.clone(),
            base_claim_rewards_result.created_with_merge,
            base_claim_rewards_result.storage_cache,
        );

        (output_farm_token_payment, locked_rewards_payment).into()
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> ExitFarmWithPartialPosResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);

        let payment = self.call_value().single_esdt();

        let migrated_amount = self.migrate_old_farm_positions(&orig_caller);

        let exit_farm_result = self.exit_farm::<NoMintWrapper<Self>>(orig_caller.clone(), payment);

        self.decrease_old_farm_positions(migrated_amount, &orig_caller);

        let rewards = exit_farm_result.rewards;
        self.send_payment_non_zero(&caller, &exit_farm_result.farming_tokens);

        let locked_rewards_payment = self.send_to_lock_contract_non_zero(
            rewards.token_identifier.clone(),
            rewards.amount,
            caller,
            orig_caller.clone(),
        );

        (exit_farm_result.farming_tokens, locked_rewards_payment).into()
    }

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens_endpoint(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> DoubleMultiPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);

        self.migrate_old_farm_positions(&orig_caller);
        let boosted_rewards = self.claim_only_boosted_payment(&orig_caller);

        let merged_farm_token = self.merge_and_update_farm_tokens(orig_caller.clone());

        self.send_payment_non_zero(&caller, &merged_farm_token);
        let locked_rewards_payment = self.send_to_lock_contract_non_zero(
            self.reward_token_id().get(),
            boosted_rewards,
            caller,
            orig_caller,
        );

        (merged_farm_token, locked_rewards_payment).into()
    }

    fn merge_and_update_farm_tokens(&self, orig_caller: ManagedAddress) -> EsdtTokenPayment {
        let mut output_attributes =
            self.merge_and_return_attributes::<NoMintWrapper<Self>>(&orig_caller);
        output_attributes.original_owner = orig_caller;

        let new_token_amount = output_attributes.get_total_supply();
        self.farm_token()
            .nft_create(new_token_amount, &output_attributes)
    }

    #[endpoint(claimBoostedRewards)]
    fn claim_boosted_rewards(
        &self,
        opt_user: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        let user = match &opt_user {
            OptionalValue::Some(user) => user,
            OptionalValue::None => &caller,
        };
        let user_total_farm_position = self.get_user_total_farm_position(user);
        if user != &caller {
            require!(
                user_total_farm_position.allow_external_claim_boosted_rewards,
                "Cannot claim rewards for this address"
            );
        }

        let boosted_rewards = self.claim_only_boosted_payment(user);
        self.send_to_lock_contract_non_zero(
            self.reward_token_id().get(),
            boosted_rewards,
            user.clone(),
            user.clone(),
        )
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.start_produce_rewards();
    }

    #[endpoint(endProduceRewards)]
    fn end_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.end_produce_rewards::<NoMintWrapper<Self>>();
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards_endpoint(&self, per_block_amount: BigUint) {
        self.require_caller_has_admin_permissions();
        self.set_per_block_rewards::<NoMintWrapper<Self>>(per_block_amount);
    }

    #[endpoint(setBoostedYieldsRewardsPercentage)]
    fn set_boosted_yields_rewards_percentage(&self, percentage: u64) {
        self.require_caller_has_admin_permissions();
        require!(percentage <= MAX_PERCENT, "Invalid percentage");

        let mut storage_cache = StorageCache::new(self);
        NoMintWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        self.boosted_yields_rewards_percentage().set(percentage);
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        user: ManagedAddress,
        farm_token_amount: BigUint,
        attributes: FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        self.require_queried();

        let mut storage_cache = StorageCache::new(self);
        NoMintWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        NoMintWrapper::<Self>::calculate_rewards(
            self,
            &user,
            &farm_token_amount,
            &attributes,
            &storage_cache,
        )
    }

    fn send_to_lock_contract_non_zero(
        &self,
        token_id: TokenIdentifier,
        amount: BigUint,
        destination_address: ManagedAddress,
        energy_address: ManagedAddress,
    ) -> EsdtTokenPayment {
        if amount == 0 {
            let locked_token_id = self.get_locked_token_id();
            return EsdtTokenPayment::new(locked_token_id, 0, amount);
        }

        self.lock_virtual(token_id, amount, destination_address, energy_address)
    }
}

pub struct NoMintWrapper<T: BaseFunctionsModule + farm_boosted_yields::FarmBoostedYieldsModule> {
    _phantom: PhantomData<T>,
}

impl<T> FarmContract for NoMintWrapper<T>
where
    T: BaseFunctionsModule + farm_boosted_yields::FarmBoostedYieldsModule,
{
    type FarmSc = T;
    type AttributesType = FarmTokenAttributes<<Self::FarmSc as ContractBase>::Api>;

    fn mint_rewards(
        _sc: &Self::FarmSc,
        _token_id: &TokenIdentifier<<Self::FarmSc as ContractBase>::Api>,
        _amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) {
    }

    fn generate_aggregated_rewards(
        sc: &Self::FarmSc,
        storage_cache: &mut StorageCache<Self::FarmSc>,
    ) {
        let total_reward = Self::mint_per_block_rewards(sc, &storage_cache.reward_token_id);
        if total_reward > 0u64 {
            storage_cache.reward_reserve += &total_reward;
            let split_rewards = sc.take_reward_slice(total_reward);

            if storage_cache.farm_token_supply != 0u64 {
                let increase = (&split_rewards.base_farm * &storage_cache.division_safety_constant)
                    / &storage_cache.farm_token_supply;
                storage_cache.reward_per_share += &increase;
            }
        }
    }

    fn calculate_rewards(
        sc: &Self::FarmSc,
        caller: &ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        farm_token_amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
        token_attributes: &Self::AttributesType,
        storage_cache: &StorageCache<Self::FarmSc>,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        Wrapper::<T>::calculate_rewards(
            sc,
            caller,
            farm_token_amount,
            token_attributes,
            storage_cache,
        )
    }

    fn get_exit_penalty(
        sc: &Self::FarmSc,
        total_exit_amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
        token_attributes: &Self::AttributesType,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        Wrapper::<T>::get_exit_penalty(sc, total_exit_amount, token_attributes)
    }

    fn apply_penalty(
        sc: &Self::FarmSc,
        total_exit_amount: &mut BigUint<<Self::FarmSc as ContractBase>::Api>,
        token_attributes: &Self::AttributesType,
        storage_cache: &StorageCache<Self::FarmSc>,
    ) {
        Wrapper::<T>::apply_penalty(sc, total_exit_amount, token_attributes, storage_cache)
    }
}
