multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Epoch;
use contexts::storage_cache::StorageCache;
use fixed_supply_token::FixedSupplyToken;

use crate::{base_impl_wrapper::FarmStakingWrapper, token_attributes::UnbondSftAttributes};

#[derive(TopDecode, TopEncode, NestedEncode, NestedDecode)]
pub struct OldUnbondAttributes {
    pub unlock_epoch: Epoch,
}

#[multiversx_sc::module]
pub trait UnbondFarmModule:
    crate::custom_rewards::CustomRewardsModule
    + crate::unbond_token::UnbondTokenModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + sc_whitelist_module::SCWhitelistModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + utils::UtilsModule
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
    #[payable("*")]
    #[endpoint(unbondFarm)]
    fn unbond_farm(&self) -> EsdtTokenPayment {
        let storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);

        let farm_token_mapper = self.farm_token();
        let unbond_token_mapper = self.unbond_token();

        let payment = self.call_value().single_esdt();
        let unlock_epoch = if &payment.token_identifier == unbond_token_mapper.get_token_id_ref() {
            let attributes: UnbondSftAttributes<Self::Api> =
                unbond_token_mapper.get_token_attributes(payment.token_nonce);

            unbond_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

            attributes.unlock_epoch
        } else if &payment.token_identifier == farm_token_mapper.get_token_id_ref() {
            let old_attributes: OldUnbondAttributes =
                farm_token_mapper.get_token_attributes(payment.token_nonce);

            farm_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

            old_attributes.unlock_epoch
        } else {
            sc_panic!("Invalid payment token")
        };

        let current_epoch = self.blockchain().get_block_epoch();
        require!(current_epoch >= unlock_epoch, "Unbond period not over");

        let caller = self.blockchain().get_caller();
        let farming_tokens =
            EsdtTokenPayment::new(storage_cache.farming_token_id.clone(), 0, payment.amount);
        self.send_payment_non_zero(&caller, &farming_tokens);

        farming_tokens
    }

    #[payable("*")]
    #[endpoint(cancelUnbond)]
    fn cancel_unbond(&self) -> EsdtTokenPayment {
        let unbond_token_mapper = self.unbond_token();
        let payment = self.call_value().single_esdt();
        unbond_token_mapper.require_same_token(&payment.token_identifier);

        let unbond_attributes: UnbondSftAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(&payment, &unbond_token_mapper);

        unbond_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        let caller = self.blockchain().get_caller();
        let total_farming_tokens = unbond_attributes.original_attributes.get_total_supply();
        let farming_token_id = self.farming_token_id().get();
        let farming_token_payment =
            EsdtTokenPayment::new(farming_token_id, 0, total_farming_tokens.clone());
        let enter_result = self.enter_farm_base_no_token_create::<FarmStakingWrapper<Self>>(
            caller.clone(),
            ManagedVec::from_single_item(farming_token_payment),
        );

        let mut new_attributes = enter_result.new_farm_token.attributes;
        new_attributes.compounded_reward = unbond_attributes.original_attributes.compounded_reward;
        new_attributes.original_owner = caller.clone();

        let total_farm_tokens = new_attributes.get_total_supply();

        // TODO: Event

        self.farm_token()
            .nft_create_and_send(&caller, total_farm_tokens, &new_attributes)
    }
}
