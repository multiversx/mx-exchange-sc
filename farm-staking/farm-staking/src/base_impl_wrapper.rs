multiversx_sc::imports!();

use core::marker::PhantomData;

use common_structs::FarmToken;
use contexts::storage_cache::StorageCache;
use farm_base_impl::base_traits_impl::FarmContract;

use crate::token_attributes::StakingFarmTokenAttributes;

pub trait FarmStakingTraits =
    crate::custom_rewards::CustomRewardsModule
        + rewards::RewardsModule
        + config::ConfigModule
        + farm_token::FarmTokenModule
        + pausable::PausableModule
        + permissions_module::PermissionsModule
        + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
        + farm_boosted_yields::FarmBoostedYieldsModule;

pub struct FarmStakingWrapper<T>
where
    T:,
{
    _phantom: PhantomData<T>,
}

impl<T> FarmStakingWrapper<T>
where
    T: FarmStakingTraits,
{
    pub fn calculate_base_farm_rewards(
        farm_token_amount: &BigUint<<<Self as FarmContract>::FarmSc as ContractBase>::Api>,
        token_attributes: &<Self as FarmContract>::AttributesType,
        storage_cache: &StorageCache<<Self as FarmContract>::FarmSc>,
    ) -> BigUint<<<Self as FarmContract>::FarmSc as ContractBase>::Api> {
        let token_rps = token_attributes.get_reward_per_share();
        if storage_cache.reward_per_share > token_rps {
            let rps_diff = &storage_cache.reward_per_share - &token_rps;
            farm_token_amount * &rps_diff / &storage_cache.division_safety_constant
        } else {
            BigUint::zero()
        }
    }

    pub fn calculate_boosted_rewards(
        sc: &<Self as FarmContract>::FarmSc,
        caller: &ManagedAddress<<<Self as FarmContract>::FarmSc as ContractBase>::Api>,
        token_attributes: &<Self as FarmContract>::AttributesType,
        farm_token_amount: BigUint<<<Self as FarmContract>::FarmSc as ContractBase>::Api>,
    ) -> BigUint<<<Self as FarmContract>::FarmSc as ContractBase>::Api> {
        if &token_attributes.original_owner != caller {
            sc.update_energy_and_progress(caller);

            return BigUint::zero();
        }

        sc.claim_boosted_yields_rewards(caller, farm_token_amount)
    }
}

impl<T> FarmContract for FarmStakingWrapper<T>
where
    T: FarmStakingTraits,
{
    type FarmSc = T;
    type AttributesType = StakingFarmTokenAttributes<<Self::FarmSc as ContractBase>::Api>;

    fn mint_rewards(
        _sc: &Self::FarmSc,
        _token_id: &TokenIdentifier<<Self::FarmSc as ContractBase>::Api>,
        _amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) {
    }

    fn mint_per_block_rewards(
        sc: &Self::FarmSc,
        _token_id: &TokenIdentifier<<Self::FarmSc as ContractBase>::Api>,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        let current_block_nonce = sc.blockchain().get_block_nonce();
        let last_reward_nonce = sc.last_reward_block_nonce().get();

        if current_block_nonce <= last_reward_nonce {
            return BigUint::zero();
        }

        let extra_rewards_unbounded =
            Self::calculate_per_block_rewards(sc, current_block_nonce, last_reward_nonce);

        let farm_token_supply = sc.farm_token_supply().get();
        let extra_rewards_apr_bounded_per_block = sc.get_amount_apr_bounded(&farm_token_supply);

        let block_nonce_diff = current_block_nonce - last_reward_nonce;
        let extra_rewards_apr_bounded = extra_rewards_apr_bounded_per_block * block_nonce_diff;

        sc.last_reward_block_nonce().set(current_block_nonce);

        core::cmp::min(extra_rewards_unbounded, extra_rewards_apr_bounded)
    }

    fn generate_aggregated_rewards(
        sc: &Self::FarmSc,
        storage_cache: &mut StorageCache<Self::FarmSc>,
    ) {
        let accumulated_rewards_mapper = sc.accumulated_rewards();
        let mut accumulated_rewards = accumulated_rewards_mapper.get();
        let reward_capacity = sc.reward_capacity().get();
        let remaining_rewards = &reward_capacity - &accumulated_rewards;

        let mut total_reward = Self::mint_per_block_rewards(sc, &storage_cache.reward_token_id);
        total_reward = core::cmp::min(total_reward, remaining_rewards);
        if total_reward == 0 {
            return;
        }

        storage_cache.reward_reserve += &total_reward;
        accumulated_rewards += &total_reward;
        accumulated_rewards_mapper.set(&accumulated_rewards);

        let split_rewards = sc.take_reward_slice(total_reward);
        if storage_cache.farm_token_supply > 0 {
            let increase = (&split_rewards.base_farm * &storage_cache.division_safety_constant)
                / &storage_cache.farm_token_supply;
            storage_cache.reward_per_share += &increase;
        }
    }

    fn calculate_rewards(
        sc: &Self::FarmSc,
        caller: &ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        farm_token_amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
        token_attributes: &Self::AttributesType,
        storage_cache: &StorageCache<Self::FarmSc>,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        let base_farm_reward =
            Self::calculate_base_farm_rewards(farm_token_amount, token_attributes, storage_cache);
        let boosted_yield_rewards = Self::calculate_boosted_rewards(
            sc,
            caller,
            token_attributes,
            farm_token_amount.clone(),
        );

        base_farm_reward + boosted_yield_rewards
    }

    fn create_enter_farm_initial_attributes(
        _sc: &Self::FarmSc,
        caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        farming_token_amount: BigUint<<Self::FarmSc as ContractBase>::Api>,
        current_reward_per_share: BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) -> Self::AttributesType {
        StakingFarmTokenAttributes {
            reward_per_share: current_reward_per_share,
            compounded_reward: BigUint::zero(),
            current_farm_amount: farming_token_amount,
            original_owner: caller,
        }
    }

    fn create_claim_rewards_initial_attributes(
        _sc: &Self::FarmSc,
        caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        first_token_attributes: Self::AttributesType,
        current_reward_per_share: BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) -> Self::AttributesType {
        StakingFarmTokenAttributes {
            reward_per_share: current_reward_per_share,
            compounded_reward: first_token_attributes.compounded_reward,
            current_farm_amount: first_token_attributes.current_farm_amount,
            original_owner: caller,
        }
    }

    fn create_compound_rewards_initial_attributes(
        _sc: &Self::FarmSc,
        caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        first_token_attributes: Self::AttributesType,
        current_reward_per_share: BigUint<<Self::FarmSc as ContractBase>::Api>,
        reward: &BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) -> Self::AttributesType {
        let new_pos_compounded_reward = first_token_attributes.compounded_reward + reward;
        let new_pos_current_farm_amount = first_token_attributes.current_farm_amount + reward;
        StakingFarmTokenAttributes {
            reward_per_share: current_reward_per_share,
            compounded_reward: new_pos_compounded_reward,
            current_farm_amount: new_pos_current_farm_amount,
            original_owner: caller,
        }
    }
}
