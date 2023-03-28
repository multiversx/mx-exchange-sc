multiversx_sc::imports!();

use common_structs::{FarmToken, FarmTokenAttributes, Nonce};
use config::ConfigModule;
use contexts::storage_cache::StorageCache;
use core::marker::PhantomData;
use fixed_supply_token::FixedSupplyToken;
use mergeable::Mergeable;
use rewards::RewardsModule;

pub trait AllBaseFarmImplTraits =
    rewards::RewardsModule
        + config::ConfigModule
        + farm_token::FarmTokenModule
        + permissions_module::PermissionsModule
        + pausable::PausableModule
        + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule;

pub trait FarmContract {
    type FarmSc: AllBaseFarmImplTraits;

    type AttributesType: 'static
        + Clone
        + TopEncode
        + TopDecode
        + NestedEncode
        + NestedDecode
        + Mergeable<<Self::FarmSc as ContractBase>::Api>
        + FixedSupplyToken<<Self::FarmSc as ContractBase>::Api>
        + FarmToken<<Self::FarmSc as ContractBase>::Api>
        + From<FarmTokenAttributes<<Self::FarmSc as ContractBase>::Api>>
        + Into<FarmTokenAttributes<<Self::FarmSc as ContractBase>::Api>> =
        FarmTokenAttributes<<Self::FarmSc as ContractBase>::Api>;

    #[inline]
    fn mint_rewards(
        sc: &Self::FarmSc,
        token_id: &TokenIdentifier<<Self::FarmSc as ContractBase>::Api>,
        amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) {
        sc.send().esdt_local_mint(token_id, 0, amount);
    }

    fn calculate_per_block_rewards(
        sc: &Self::FarmSc,
        current_block_nonce: Nonce,
        last_reward_block_nonce: Nonce,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        if current_block_nonce <= last_reward_block_nonce || !sc.produces_per_block_rewards() {
            return BigUint::zero();
        }

        let per_block_reward = sc.per_block_reward_amount().get();
        let block_nonce_diff = current_block_nonce - last_reward_block_nonce;

        per_block_reward * block_nonce_diff
    }

    fn mint_per_block_rewards(
        sc: &Self::FarmSc,
        token_id: &TokenIdentifier<<Self::FarmSc as ContractBase>::Api>,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        let current_block_nonce = sc.blockchain().get_block_nonce();
        let last_reward_nonce = sc.last_reward_block_nonce().get();
        if current_block_nonce > last_reward_nonce {
            let to_mint =
                Self::calculate_per_block_rewards(sc, current_block_nonce, last_reward_nonce);
            if to_mint != 0 {
                Self::mint_rewards(sc, token_id, &to_mint);
            }

            sc.last_reward_block_nonce().set(current_block_nonce);

            to_mint
        } else {
            BigUint::zero()
        }
    }

    fn generate_aggregated_rewards(
        sc: &Self::FarmSc,
        storage_cache: &mut StorageCache<Self::FarmSc>,
    ) {
        let total_reward = Self::mint_per_block_rewards(sc, &storage_cache.reward_token_id);
        if total_reward > 0u64 {
            storage_cache.reward_reserve += &total_reward;

            if storage_cache.farm_token_supply != 0u64 {
                let increase = (&total_reward * &storage_cache.division_safety_constant)
                    / &storage_cache.farm_token_supply;
                storage_cache.reward_per_share += &increase;
            }
        }
    }

    fn calculate_rewards(
        _sc: &Self::FarmSc,
        _caller: &ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        farm_token_amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
        token_attributes: &Self::AttributesType,
        storage_cache: &StorageCache<Self::FarmSc>,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        let token_rps = token_attributes.get_reward_per_share();
        if storage_cache.reward_per_share > token_rps {
            let rps_diff = &storage_cache.reward_per_share - &token_rps;
            farm_token_amount * &rps_diff / &storage_cache.division_safety_constant
        } else {
            BigUint::zero()
        }
    }

    fn create_enter_farm_initial_attributes(
        sc: &Self::FarmSc,
        caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        farming_token_amount: BigUint<<Self::FarmSc as ContractBase>::Api>,
        current_reward_per_share: BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) -> Self::AttributesType {
        let current_epoch = sc.blockchain().get_block_epoch();
        let attributes = FarmTokenAttributes {
            reward_per_share: current_reward_per_share,
            entering_epoch: current_epoch,
            compounded_reward: BigUint::zero(),
            current_farm_amount: farming_token_amount,
            original_owner: caller,
        };

        attributes.into()
    }

    fn create_claim_rewards_initial_attributes(
        _sc: &Self::FarmSc,
        caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        first_token_attributes: Self::AttributesType,
        current_reward_per_share: BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) -> Self::AttributesType {
        let initial_attributes: FarmTokenAttributes<<Self::FarmSc as ContractBase>::Api> =
            first_token_attributes.into();

        let net_current_farm_amount = initial_attributes.get_total_supply();
        let new_attributes = FarmTokenAttributes {
            reward_per_share: current_reward_per_share,
            entering_epoch: initial_attributes.entering_epoch,
            compounded_reward: initial_attributes.compounded_reward,
            current_farm_amount: net_current_farm_amount,
            original_owner: caller,
        };

        new_attributes.into()
    }

    fn create_compound_rewards_initial_attributes(
        sc: &Self::FarmSc,
        caller: ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        first_token_attributes: Self::AttributesType,
        current_reward_per_share: BigUint<<Self::FarmSc as ContractBase>::Api>,
        reward: &BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) -> Self::AttributesType {
        let initial_attributes: FarmTokenAttributes<<Self::FarmSc as ContractBase>::Api> =
            first_token_attributes.into();

        let current_epoch = sc.blockchain().get_block_epoch();
        let new_pos_compounded_reward = initial_attributes.compounded_reward + reward;
        let new_pos_current_farm_amount = initial_attributes.current_farm_amount + reward;
        let new_attributes = FarmTokenAttributes {
            reward_per_share: current_reward_per_share,
            entering_epoch: current_epoch,
            compounded_reward: new_pos_compounded_reward,
            current_farm_amount: new_pos_current_farm_amount,
            original_owner: caller,
        };

        new_attributes.into()
    }

    fn get_exit_penalty(
        _sc: &Self::FarmSc,
        _total_exit_amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
        _token_attributes: &Self::AttributesType,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        BigUint::zero()
    }

    fn apply_penalty(
        _sc: &Self::FarmSc,
        _total_exit_amount: &mut BigUint<<Self::FarmSc as ContractBase>::Api>,
        _token_attributes: &Self::AttributesType,
        _storage_cache: &StorageCache<Self::FarmSc>,
    ) {
    }
}

pub struct DefaultFarmWrapper<T>
where
    T: AllBaseFarmImplTraits,
{
    _phantom: PhantomData<T>,
}

impl<T> FarmContract for DefaultFarmWrapper<T>
where
    T: AllBaseFarmImplTraits,
{
    type FarmSc = T;
}
