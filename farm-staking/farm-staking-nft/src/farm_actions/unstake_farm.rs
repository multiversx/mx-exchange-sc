multiversx_sc::imports!();

use common_structs::PaymentsVec;
use contexts::{
    exit_farm_context::ExitFarmContext,
    storage_cache::{FarmContracTraitBounds, StorageCache},
};

use crate::{
    common::result_types::UnstakeRewardsResultType,
    common::token_attributes::{StakingFarmNftTokenAttributes, UnbondSftAttributes},
};

const NFT_AMOUNT: u32 = 1;

pub struct InternalExitFarmResult<'a, C>
where
    C: FarmContracTraitBounds,
{
    pub context: ExitFarmContext<C::Api, StakingFarmNftTokenAttributes<C::Api>>,
    pub storage_cache: StorageCache<'a, C>,
    pub token_parts: PaymentsVec<C::Api>,
    pub reward_payment: EsdtTokenPayment<C::Api>,
}

#[multiversx_sc::module]
pub trait UnstakeFarmModule:
    crate::custom_rewards::CustomRewardsModule
    + super::claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
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
    + crate::common::token_info::TokenInfoModule
    + crate::common::unbond_token::UnbondTokenModule
    + crate::common::custom_events::CustomEventsModule
{
    #[payable("*")]
    #[endpoint(unstakeFarm)]
    fn unstake_farm(&self) -> UnstakeRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();

        let mut exit_result = self.exit_farm_base(caller.clone(), payment);
        let reward_nonce = self.reward_nonce().get();
        exit_result.reward_payment.token_nonce = reward_nonce;

        let unbond_farm_token = self.create_unbond_tokens(exit_result.token_parts);

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

        UnstakeRewardsResultType {
            unbond_farm_token,
            reward_payment: exit_result.reward_payment,
        }
    }

    fn create_unbond_tokens(
        &self,
        farming_token_parts: PaymentsVec<Self::Api>,
    ) -> EsdtTokenPayment {
        let min_unbond_epochs = self.min_unbond_epochs().get();
        let current_epoch = self.blockchain().get_block_epoch();

        self.unbond_token().nft_create(
            BigUint::from(NFT_AMOUNT),
            &UnbondSftAttributes {
                unlock_epoch: current_epoch + min_unbond_epochs,
                farming_token_parts,
            },
        )
    }

    fn exit_farm_base(
        &self,
        caller: ManagedAddress,
        payment: EsdtTokenPayment<Self::Api>,
    ) -> InternalExitFarmResult<Self> {
        let mut storage_cache = StorageCache::new(self);
        self.validate_contract_state(storage_cache.contract_state, &storage_cache.farm_token_id);

        let exit_farm_context =
            ExitFarmContext::<Self::Api, StakingFarmNftTokenAttributes<Self::Api>>::new(
                payment.clone(),
                &storage_cache.farm_token_id,
                self.blockchain(),
            );

        self.generate_aggregated_rewards(&mut storage_cache);

        let farm_token_amount = &exit_farm_context.farm_token.payment.amount;
        let token_attributes = self.into_part(
            exit_farm_context.farm_token.attributes.clone(),
            &exit_farm_context.farm_token.payment,
        );

        let reward = self.calculate_rewards(
            &caller,
            farm_token_amount,
            &token_attributes,
            &storage_cache,
        );
        storage_cache.reward_reserve -= &reward;

        self.decrease_user_farm_position(&payment);

        let reward_payment =
            EsdtTokenPayment::new(storage_cache.reward_token_id.clone(), 0, reward);

        let farm_token_payment = &exit_farm_context.farm_token.payment;
        self.send().esdt_local_burn(
            &farm_token_payment.token_identifier,
            farm_token_payment.token_nonce,
            &farm_token_payment.amount,
        );

        storage_cache.farm_token_supply -= &payment.amount;

        InternalExitFarmResult {
            context: exit_farm_context,
            token_parts: token_attributes.farming_token_parts,
            reward_payment,
            storage_cache,
        }
    }
}
