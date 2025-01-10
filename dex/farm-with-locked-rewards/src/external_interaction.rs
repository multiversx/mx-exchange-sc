multiversx_sc::imports!();

use common_structs::FarmTokenAttributes;
use farm::{
    base_functions::{self, ClaimRewardsResultType},
    exit_penalty, EnterFarmResultType,
};

use crate::NoMintWrapper;

#[multiversx_sc::module]
pub trait ExternalInteractionsModule:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + permissions_hub_module::PermissionsHubModule
    + original_owner_helper::OriginalOwnerHelperModule
    + sc_whitelist_module::SCWhitelistModule
    + events::EventsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + base_functions::BaseFunctionsModule
    + exit_penalty::ExitPenaltyModule
    + locking_module::lock_with_energy_module::LockWithEnergyModule
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
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(enterFarmOnBehalf)]
    fn enter_farm_on_behalf(&self, user: ManagedAddress) -> EnterFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_user_whitelisted(&user, &caller);

        let payments = self.get_non_empty_payments();
        let farm_token_mapper = self.farm_token();
        self.check_additional_payments_original_owner::<FarmTokenAttributes<Self::Api>>(
            &user,
            &payments,
            &farm_token_mapper,
        );

        let boosted_rewards = self.claim_only_boosted_payment(&user);
        let new_farm_token = self.enter_farm::<NoMintWrapper<Self>>(user.clone());
        self.send_payment_non_zero(&caller, &new_farm_token);

        let locked_rewards_payment = if boosted_rewards == 0 {
            let locked_token_id = self.get_locked_token_id();
            EsdtTokenPayment::new(locked_token_id, 0, boosted_rewards)
        } else {
            self.lock_virtual(
                self.reward_token_id().get(),
                boosted_rewards,
                user.clone(),
                user.clone(),
            )
        };

        self.update_energy_and_progress(&user);

        (new_farm_token, locked_rewards_payment).into()
    }

    #[payable("*")]
    #[endpoint(claimRewardsOnBehalf)]
    fn claim_rewards_on_behalf(&self) -> ClaimRewardsResultType<Self::Api> {
        let payments = self.get_non_empty_payments();
        let farm_token_mapper = self.farm_token();
        let caller = self.blockchain().get_caller();
        let user = self.check_and_return_original_owner::<FarmTokenAttributes<Self::Api>>(
            &payments,
            &farm_token_mapper,
        );
        self.require_user_whitelisted(&user, &caller);

        let claim_rewards_result = self.claim_rewards::<NoMintWrapper<Self>>(user.clone());

        self.send_payment_non_zero(&caller, &claim_rewards_result.new_farm_token);

        let rewards_payment = claim_rewards_result.rewards;
        let locked_rewards_payment = if rewards_payment.amount == 0 {
            let locked_token_id = self.get_locked_token_id();
            EsdtTokenPayment::new(locked_token_id, 0, rewards_payment.amount)
        } else {
            self.lock_virtual(
                rewards_payment.token_identifier,
                rewards_payment.amount,
                user.clone(),
                user,
            )
        };

        (claim_rewards_result.new_farm_token, locked_rewards_payment).into()
    }
}
