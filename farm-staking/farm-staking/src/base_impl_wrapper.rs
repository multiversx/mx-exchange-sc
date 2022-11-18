elrond_wasm::imports!();

use core::marker::PhantomData;

use farm_base_impl::base_traits_impl::FarmContract;

use crate::token_attributes::StakingFarmTokenAttributes;

pub trait FarmStakingTraits =
    crate::custom_rewards::CustomRewardsModule
        + rewards::RewardsModule
        + config::ConfigModule
        + farm_token::FarmTokenModule
        + pausable::PausableModule
        + permissions_module::PermissionsModule
        + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule;

pub struct FarmStakingWrapper<T>
where
    T:,
{
    _phantom: PhantomData<T>,
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

        if current_block_nonce > last_reward_nonce {
            let extra_rewards_unbounded =
                Self::calculate_per_block_rewards(sc, current_block_nonce, last_reward_nonce);

            let farm_token_supply = sc.farm_token_supply().get();
            let extra_rewards_apr_bounded_per_block = sc.get_amount_apr_bounded(&farm_token_supply);

            let block_nonce_diff = current_block_nonce - last_reward_nonce;
            let extra_rewards_apr_bounded = extra_rewards_apr_bounded_per_block * block_nonce_diff;

            sc.last_reward_block_nonce().set(current_block_nonce);

            core::cmp::min(extra_rewards_unbounded, extra_rewards_apr_bounded)
        } else {
            BigUint::zero()
        }
    }

    fn create_enter_farm_initial_attributes(
        _sc: &Self::FarmSc,
        _caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        farming_token_amount: BigUint<<Self::FarmSc as ContractBase>::Api>,
        current_reward_per_share: BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) -> Self::AttributesType {
        StakingFarmTokenAttributes {
            reward_per_share: current_reward_per_share,
            compounded_reward: BigUint::zero(),
            current_farm_amount: farming_token_amount,
        }
    }

    fn create_claim_rewards_initial_attributes(
        _sc: &Self::FarmSc,
        _caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        first_token_attributes: Self::AttributesType,
        current_reward_per_share: BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) -> Self::AttributesType {
        StakingFarmTokenAttributes {
            reward_per_share: current_reward_per_share,
            compounded_reward: first_token_attributes.compounded_reward,
            current_farm_amount: first_token_attributes.current_farm_amount,
        }
    }

    fn create_compound_rewards_initial_attributes(
        _sc: &Self::FarmSc,
        _caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
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
        }
    }
}
