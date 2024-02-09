multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::{Epoch, Nonce, PaymentsVec};
use contexts::storage_cache::StorageCache;

use crate::common::token_attributes::{
    PartialStakingFarmNftTokenAttributes, StakingFarmNftTokenAttributes,
};

pub const MAX_PERCENT: u64 = 10_000;
pub const BLOCKS_IN_YEAR: u64 = 31_536_000 / 6; // seconds_in_year / 6_seconds_per_block

#[multiversx_sc::module]
pub trait CustomRewardsModule:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + utils::UtilsModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
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
    fn get_amount_apr_bounded(&self, amount: &BigUint) -> BigUint {
        let max_apr = self.max_annual_percentage_rewards().get();
        amount * &max_apr / MAX_PERCENT / BLOCKS_IN_YEAR
    }

    fn calculate_base_farm_rewards(
        &self,
        farm_token_amount: &BigUint,
        token_attributes: &PartialStakingFarmNftTokenAttributes<Self::Api>,
        storage_cache: &StorageCache<Self>,
    ) -> BigUint {
        let token_rps = token_attributes.reward_per_share.clone();
        if storage_cache.reward_per_share > token_rps {
            let rps_diff = &storage_cache.reward_per_share - &token_rps;
            farm_token_amount * &rps_diff / &storage_cache.division_safety_constant
        } else {
            BigUint::zero()
        }
    }

    fn calculate_boosted_rewards(&self, caller: &ManagedAddress) -> BigUint {
        let user_total_farm_position = self.get_user_total_farm_position(caller);
        let user_farm_position = user_total_farm_position.total_farm_position;
        if user_farm_position == 0 {
            return BigUint::zero();
        }

        self.claim_boosted_yields_rewards(caller, user_farm_position)
    }

    fn calculate_per_block_rewards(
        &self,
        current_block_nonce: Nonce,
        last_reward_block_nonce: Nonce,
    ) -> BigUint {
        if current_block_nonce <= last_reward_block_nonce || !self.produces_per_block_rewards() {
            return BigUint::zero();
        }

        let per_block_reward = self.per_block_reward_amount().get();
        let block_nonce_diff = current_block_nonce - last_reward_block_nonce;

        per_block_reward * block_nonce_diff
    }

    fn mint_per_block_rewards(&self) -> BigUint {
        let current_block_nonce = self.blockchain().get_block_nonce();
        let last_reward_nonce = self.last_reward_block_nonce().get();

        if current_block_nonce <= last_reward_nonce {
            return BigUint::zero();
        }

        let extra_rewards_unbounded =
            self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);

        let farm_token_supply = self.farm_token_supply().get();
        let extra_rewards_apr_bounded_per_block = self.get_amount_apr_bounded(&farm_token_supply);

        let block_nonce_diff = current_block_nonce - last_reward_nonce;
        let extra_rewards_apr_bounded = extra_rewards_apr_bounded_per_block * block_nonce_diff;

        self.last_reward_block_nonce().set(current_block_nonce);

        core::cmp::min(extra_rewards_unbounded, extra_rewards_apr_bounded)
    }

    fn generate_aggregated_rewards(&self, storage_cache: &mut StorageCache<Self>) {
        let accumulated_rewards_mapper = self.accumulated_rewards();
        let mut accumulated_rewards = accumulated_rewards_mapper.get();
        let reward_capacity = self.reward_capacity().get();
        let remaining_rewards = &reward_capacity - &accumulated_rewards;

        let mut total_reward = self.mint_per_block_rewards();
        total_reward = core::cmp::min(total_reward, remaining_rewards);
        if total_reward == 0 {
            return;
        }

        storage_cache.reward_reserve += &total_reward;
        accumulated_rewards += &total_reward;
        accumulated_rewards_mapper.set(&accumulated_rewards);

        let split_rewards = self.take_reward_slice(total_reward);
        if storage_cache.farm_token_supply > 0 {
            let increase = (&split_rewards.base_farm * &storage_cache.division_safety_constant)
                / &storage_cache.farm_token_supply;
            storage_cache.reward_per_share += &increase;
        }
    }

    fn calculate_rewards(
        &self,
        caller: &ManagedAddress,
        farm_token_amount: &BigUint,
        token_attributes: &PartialStakingFarmNftTokenAttributes<Self::Api>,
        storage_cache: &StorageCache<Self>,
    ) -> BigUint {
        let base_farm_reward =
            self.calculate_base_farm_rewards(farm_token_amount, token_attributes, storage_cache);
        let boosted_yield_rewards = self.calculate_boosted_rewards(caller);

        base_farm_reward + boosted_yield_rewards
    }

    fn create_enter_farm_initial_attributes(
        &self,
        caller: ManagedAddress,
        farming_token_amount: BigUint,
        current_reward_per_share: BigUint,
    ) -> PartialStakingFarmNftTokenAttributes<Self::Api> {
        PartialStakingFarmNftTokenAttributes {
            reward_per_share: current_reward_per_share,
            compounded_reward: BigUint::zero(),
            current_farm_amount: farming_token_amount,
            original_owner: caller,
            farming_token_parts: PaymentsVec::new(),
        }
    }

    fn create_claim_rewards_initial_attributes(
        &self,
        caller: ManagedAddress,
        first_token_attributes: PartialStakingFarmNftTokenAttributes<Self::Api>,
        current_reward_per_share: BigUint,
    ) -> PartialStakingFarmNftTokenAttributes<Self::Api> {
        PartialStakingFarmNftTokenAttributes {
            reward_per_share: current_reward_per_share,
            compounded_reward: first_token_attributes.compounded_reward,
            current_farm_amount: first_token_attributes.current_farm_amount,
            original_owner: caller,
            farming_token_parts: first_token_attributes.farming_token_parts,
        }
    }

    fn create_compound_rewards_initial_attributes(
        &self,
        caller: ManagedAddress,
        first_token_attributes: PartialStakingFarmNftTokenAttributes<Self::Api>,
        current_reward_per_share: BigUint,
        reward: &BigUint,
    ) -> PartialStakingFarmNftTokenAttributes<Self::Api> {
        let new_pos_compounded_reward = first_token_attributes.compounded_reward + reward;
        let new_pos_current_farm_amount = first_token_attributes.current_farm_amount + reward;
        PartialStakingFarmNftTokenAttributes {
            reward_per_share: current_reward_per_share,
            compounded_reward: new_pos_compounded_reward,
            current_farm_amount: new_pos_current_farm_amount,
            original_owner: caller,
            farming_token_parts: first_token_attributes.farming_token_parts,
        }
    }

    fn check_and_update_user_farm_position(
        &self,
        user: &ManagedAddress,
        farm_positions: &PaymentsVec<Self::Api>,
    ) {
        let farm_token_mapper = self.farm_token();
        for farm_position in farm_positions {
            farm_token_mapper.require_same_token(&farm_position.token_identifier);

            let token_attributes: StakingFarmNftTokenAttributes<Self::Api> =
                farm_token_mapper.get_token_attributes(farm_position.token_nonce);

            if &token_attributes.original_owner != user {
                self.decrease_user_farm_position(&farm_position);
                self.increase_user_farm_position(user, &farm_position.amount);
            }
        }
    }

    fn increase_user_farm_position(
        &self,
        user: &ManagedAddress,
        increase_farm_position_amount: &BigUint,
    ) {
        let mut user_total_farm_position = self.get_user_total_farm_position(user);
        user_total_farm_position.total_farm_position += increase_farm_position_amount;
        self.user_total_farm_position(user)
            .set(user_total_farm_position);
    }

    fn decrease_user_farm_position(&self, farm_position: &EsdtTokenPayment) {
        let farm_token_mapper = self.farm_token();
        let token_attributes: StakingFarmNftTokenAttributes<Self::Api> =
            farm_token_mapper.get_token_attributes(farm_position.token_nonce);

        self.user_total_farm_position(&token_attributes.original_owner)
            .update(|user_total_farm_position| {
                if user_total_farm_position.total_farm_position > farm_position.amount {
                    user_total_farm_position.total_farm_position -= &farm_position.amount;
                } else {
                    user_total_farm_position.total_farm_position = BigUint::zero();
                }
            });
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.start_produce_rewards();
    }

    #[view(getAccumulatedRewards)]
    #[storage_mapper("accumulatedRewards")]
    fn accumulated_rewards(&self) -> SingleValueMapper<BigUint>;

    #[view(getRewardCapacity)]
    #[storage_mapper("reward_capacity")]
    fn reward_capacity(&self) -> SingleValueMapper<BigUint>;

    #[view(getAnnualPercentageRewards)]
    #[storage_mapper("annualPercentageRewards")]
    fn max_annual_percentage_rewards(&self) -> SingleValueMapper<BigUint>;

    #[view(getMinUnbondEpochs)]
    #[storage_mapper("minUnbondEpochs")]
    fn min_unbond_epochs(&self) -> SingleValueMapper<Epoch>;
}
