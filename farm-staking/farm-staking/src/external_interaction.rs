multiversx_sc::imports!();

use farm::{base_functions::ClaimRewardsResultType, EnterFarmResultType};

use crate::{
    base_impl_wrapper::FarmStakingWrapper, claim_only_boosted_staking_rewards,
    claim_stake_farm_rewards, compound_stake_farm_rewards, custom_rewards, farm_token_roles,
    stake_farm, token_attributes::StakingFarmTokenAttributes, unbond_farm, unstake_farm,
};

#[multiversx_sc::module]
pub trait ExternalInteractionsModule:
    custom_rewards::CustomRewardsModule
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
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + utils::UtilsModule
    + farm_token_roles::FarmTokenRolesModule
    + stake_farm::StakeFarmModule
    + claim_stake_farm_rewards::ClaimStakeFarmRewardsModule
    + compound_stake_farm_rewards::CompoundStakeFarmRewardsModule
    + unstake_farm::UnstakeFarmModule
    + unbond_farm::UnbondFarmModule
    + claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule
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
    #[endpoint(stakeFarmOnBehalf)]
    fn stake_farm_on_behalf(&self, user: ManagedAddress) -> EnterFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_user_whitelisted(&user, &caller);

        let payments = self.get_non_empty_payments();
        self.check_additional_payments_original_owner(&user, &payments);

        let boosted_rewards = self.claim_only_boosted_payment(&user);
        let boosted_rewards_payment =
            EsdtTokenPayment::new(self.reward_token_id().get(), 0, boosted_rewards);

        let enter_result = self.enter_farm_base::<FarmStakingWrapper<Self>>(user.clone(), payments);

        let new_farm_token = enter_result.new_farm_token.payment.clone();
        self.send_payment_non_zero(&caller, &new_farm_token);
        self.send_payment_non_zero(&user, &boosted_rewards_payment);

        self.set_farm_supply_for_current_week(&enter_result.storage_cache.farm_token_supply);

        self.update_energy_and_progress(&user);

        self.emit_enter_farm_event(
            &caller,
            enter_result.context.farming_token_payment,
            enter_result.new_farm_token,
            enter_result.created_with_merge,
            enter_result.storage_cache,
        );

        (new_farm_token, boosted_rewards_payment).into()
    }

    #[payable("*")]
    #[endpoint(claimRewardsOnBehalf)]
    fn claim_rewards_on_behalf(&self) -> ClaimRewardsResultType<Self::Api> {
        let payment = self.call_value().single_esdt();
        let user = self.check_and_return_original_owner(&payment);
        let caller = self.blockchain().get_caller();
        self.require_user_whitelisted(&user, &caller);

        let claim_result = self.claim_rewards_base_no_farm_token_mint::<FarmStakingWrapper<Self>>(
            user.clone(),
            ManagedVec::from_single_item(payment),
        );

        let mut virtual_farm_token = claim_result.new_farm_token.clone();

        self.set_farm_supply_for_current_week(&claim_result.storage_cache.farm_token_supply);

        self.update_energy_and_progress(&user);

        let new_farm_token_nonce = self.send().esdt_nft_create_compact(
            &virtual_farm_token.payment.token_identifier,
            &virtual_farm_token.payment.amount,
            &virtual_farm_token.attributes,
        );
        virtual_farm_token.payment.token_nonce = new_farm_token_nonce;

        let caller = self.blockchain().get_caller();
        self.send_payment_non_zero(&caller, &virtual_farm_token.payment);
        self.send_payment_non_zero(&user, &claim_result.rewards);

        self.emit_claim_rewards_event(
            &caller,
            claim_result.context,
            virtual_farm_token.clone(),
            claim_result.rewards.clone(),
            claim_result.created_with_merge,
            claim_result.storage_cache,
        );

        (virtual_farm_token.payment, claim_result.rewards).into()
    }

    fn check_and_return_original_owner(&self, payment: &EsdtTokenPayment) -> ManagedAddress {
        let farm_token_mapper = self.farm_token();
        let attributes: StakingFarmTokenAttributes<Self::Api> =
            farm_token_mapper.get_token_attributes(payment.token_nonce);

        require!(
            !attributes.original_owner.is_zero(),
            "Original owner could not be identified"
        );

        attributes.original_owner
    }

    fn check_additional_payments_original_owner(
        &self,
        user: &ManagedAddress,
        payments: &ManagedVec<EsdtTokenPayment>,
    ) {
        if payments.len() == 1 {
            return;
        }

        let farm_token_mapper = self.farm_token();
        let farm_token_id = farm_token_mapper.get_token_id();
        for payment in payments.into_iter() {
            if payment.token_identifier != farm_token_id {
                continue;
            }

            let attributes: StakingFarmTokenAttributes<Self::Api> =
                farm_token_mapper.get_token_attributes(payment.token_nonce);

            require!(
                user == &attributes.original_owner,
                "Provided address is not the same as the original owner"
            );
        }
    }

    fn require_user_whitelisted(&self, user: &ManagedAddress, authorized_address: &ManagedAddress) {
        let permissions_hub_address = self.permissions_hub_address().get();
        let is_whitelisted: bool = self
            .permissions_hub_proxy(permissions_hub_address)
            .is_whitelisted(user, authorized_address)
            .execute_on_dest_context();

        require!(is_whitelisted, "Caller is not whitelisted by the user");
    }

    #[only_owner]
    #[endpoint(setPermissionsHubAddress)]
    fn set_permissions_hub_address(&self, address: ManagedAddress) {
        self.permissions_hub_address().set(&address);
    }

    #[proxy]
    fn permissions_hub_proxy(
        &self,
        sc_address: ManagedAddress,
    ) -> permissions_hub::Proxy<Self::Api>;

    #[storage_mapper("permissionsHubAddress")]
    fn permissions_hub_address(&self) -> SingleValueMapper<ManagedAddress>;
}
