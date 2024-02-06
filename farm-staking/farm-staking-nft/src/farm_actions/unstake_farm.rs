multiversx_sc::imports!();

use farm::ExitFarmWithPartialPosResultType;

use crate::{
    base_impl_wrapper::FarmStakingNftWrapper, farm_hooks::hook_type::FarmHookType,
    token_attributes::UnbondSftAttributes,
};

#[multiversx_sc::module]
pub trait UnstakeFarmModule:
    crate::custom_rewards::CustomRewardsModule
    + super::claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
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
    + banned_addresses::BannedAddressModule
    + crate::farm_hooks::change_hooks::ChangeHooksModule
    + crate::farm_hooks::call_hook::CallHookModule
{
    #[payable("*")]
    #[endpoint(unstakeFarm)]
    fn unstake_farm(&self) -> ExitFarmWithPartialPosResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();

        let payments_after_hook = self.call_hook(
            FarmHookType::BeforeUnstake,
            caller.clone(),
            ManagedVec::from_single_item(payment),
            ManagedVec::new(),
        );
        let payment = payments_after_hook.get(0);

        let mut exit_result =
            self.exit_farm_base::<FarmStakingNftWrapper<Self>>(caller.clone(), payment);

        let unbond_token_amount = exit_result.farming_token_payment.amount;
        let farm_token_id = exit_result.storage_cache.farm_token_id.clone();
        let unbond_farm_token = self.create_unbond_tokens(farm_token_id, unbond_token_amount);

        let mut output_payments = ManagedVec::new();
        output_payments.push(unbond_farm_token);
        self.push_if_non_zero_payment(&mut output_payments, exit_result.reward_payment.clone());

        let mut output_payments_after_hook = self.call_hook(
            FarmHookType::AfterUnstake,
            caller.clone(),
            output_payments,
            ManagedVec::new(),
        );
        let unbond_farm_token = self.pop_first_payment(&mut output_payments_after_hook);
        exit_result.reward_payment =
            self.pop_or_return_payment(&mut output_payments_after_hook, exit_result.reward_payment);

        let caller = self.blockchain().get_caller();
        self.send_payment_non_zero(&caller, &unbond_farm_token);
        self.send_payment_non_zero(&caller, &exit_result.reward_payment);

        self.clear_user_energy_if_needed(&caller);
        self.set_farm_supply_for_current_week(&exit_result.storage_cache.farm_token_supply);

        self.emit_exit_farm_event(
            &caller,
            exit_result.context,
            unbond_farm_token.clone(),
            exit_result.reward_payment.clone(),
            exit_result.storage_cache,
        );

        (unbond_farm_token, exit_result.reward_payment).into()
    }

    fn create_unbond_tokens(
        &self,
        farm_token_id: TokenIdentifier,
        amount: BigUint,
    ) -> EsdtTokenPayment {
        let min_unbond_epochs = self.min_unbond_epochs().get();
        let current_epoch = self.blockchain().get_block_epoch();
        let nft_nonce = self.send().esdt_nft_create_compact(
            &farm_token_id,
            &amount,
            &UnbondSftAttributes {
                unlock_epoch: current_epoch + min_unbond_epochs,
            },
        );

        EsdtTokenPayment::new(farm_token_id, nft_nonce, amount)
    }
}
