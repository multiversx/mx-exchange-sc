multiversx_sc::imports!();

use farm::ExitFarmWithPartialPosResultType;
use mergeable::Mergeable;

use crate::{base_impl_wrapper::FarmStakingWrapper, token_attributes::UnbondSftAttributes};

#[multiversx_sc::module]
pub trait UnstakeFarmModule:
    crate::custom_rewards::CustomRewardsModule
    + crate::claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule
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
{
    #[payable("*")]
    #[endpoint(unstakeFarm)]
    fn unstake_farm(
        &self,
        exit_amount: BigUint,
        opt_original_caller: OptionalValue<ManagedAddress>,
    ) -> ExitFarmWithPartialPosResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let original_caller = self.get_orig_caller_from_opt(&caller, opt_original_caller);
        let payment = self.call_value().single_esdt();

        self.unstake_farm_common(original_caller, payment, exit_amount, None)
    }

    #[payable("*")]
    #[endpoint(unstakeFarmThroughProxy)]
    fn unstake_farm_through_proxy(
        &self,
        exit_amount: BigUint,
        original_caller: ManagedAddress,
    ) -> ExitFarmWithPartialPosResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        let [first_payment, second_payment] = self.call_value().multi_esdt();

        // first payment are the staking tokens, taken from the liquidity pool
        // they will be sent to the user on unbond
        let staking_token_id = self.farming_token_id().get();
        require!(
            first_payment.token_identifier == staking_token_id,
            "Invalid staking token received"
        );

        self.unstake_farm_common(
            original_caller,
            second_payment,
            exit_amount,
            Some(first_payment.amount),
        )
    }

    fn unstake_farm_common(
        &self,
        original_caller: ManagedAddress,
        mut payment: EsdtTokenPayment,
        exit_amount: BigUint,
        opt_unbond_amount: Option<BigUint>,
    ) -> ExitFarmWithPartialPosResultType<Self::Api> {
        require!(
            payment.amount >= exit_amount,
            "Exit amount is bigger than the payment amount"
        );

        let boosted_rewards_full_position =
            self.claim_only_boosted_payment(&original_caller, &payment);
        let remaining_farm_payment = EsdtTokenPayment::new(
            payment.token_identifier.clone(),
            payment.token_nonce,
            &payment.amount - &exit_amount,
        );

        payment.amount = exit_amount;

        let mut exit_result =
            self.exit_farm_base::<FarmStakingWrapper<Self>>(original_caller.clone(), payment);
        exit_result
            .reward_payment
            .merge_with(boosted_rewards_full_position);

        let unbond_token_amount =
            opt_unbond_amount.unwrap_or(exit_result.farming_token_payment.amount);
        let farm_token_id = exit_result.storage_cache.farm_token_id.clone();

        let caller = self.blockchain().get_caller();
        let unbond_farm_token =
            self.create_and_send_unbond_tokens(&caller, farm_token_id, unbond_token_amount);

        self.send_payment_non_zero(&caller, &exit_result.reward_payment);
        self.send_payment_non_zero(&caller, &remaining_farm_payment);

        self.clear_user_energy_if_needed(&original_caller, &remaining_farm_payment.amount);
        self.set_farm_supply_for_current_week(&exit_result.storage_cache.farm_token_supply);

        self.emit_exit_farm_event(
            &caller,
            exit_result.context,
            unbond_farm_token.clone(),
            exit_result.reward_payment.clone(),
            exit_result.storage_cache,
        );

        (
            unbond_farm_token,
            exit_result.reward_payment,
            remaining_farm_payment,
        )
            .into()
    }

    fn create_and_send_unbond_tokens(
        &self,
        to: &ManagedAddress,
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
        self.send()
            .direct_esdt(to, &farm_token_id, nft_nonce, &amount);

        EsdtTokenPayment::new(farm_token_id, nft_nonce, amount)
    }
}
