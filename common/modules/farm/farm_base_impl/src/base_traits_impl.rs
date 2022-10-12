elrond_wasm::imports!();

use common_structs::{FarmToken, FarmTokenAttributes};
use contexts::storage_cache::StorageCache;
use core::any::TypeId;
use elrond_wasm::elrond_codec::TopEncode;
use fixed_supply_token::FixedSupplyToken;
use mergeable::Mergeable;

pub trait AllBaseFarmImplTraits =
    rewards::RewardsModule
        + config::ConfigModule
        + token_send::TokenSendModule
        + farm_token::FarmTokenModule
        + pausable::PausableModule
        + permissions_module::PermissionsModule
        + events::EventsModule
        + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
        + crate::base_farm_init::BaseFarmInitModule
        + crate::base_farm_validation::BaseFarmValidationModule
        + crate::enter_farm::BaseEnterFarmModule
        + crate::claim_rewards::BaseClaimRewardsModule
        + crate::compound_rewards::BaseCompoundRewardsModule
        + crate::exit_farm::BaseExitFarmModule;

pub trait FarmContract: AllBaseFarmImplTraits {
    type AttributesType: 'static
        + Clone
        + TopEncode
        + TopDecode
        + NestedEncode
        + NestedDecode
        + Mergeable<Self::Api>
        + FixedSupplyToken<Self::Api>
        + FarmToken<Self::Api> = FarmTokenAttributes<Self::Api>;

    #[inline]
    fn mint_rewards(&self, token_id: &TokenIdentifier<Self::Api>, amount: &BigUint<Self::Api>) {
        self.send().esdt_local_mint(token_id, 0, amount);
    }

    fn generate_aggregated_rewards(&self, storage_cache: &mut StorageCache<Self>) {
        let total_reward =
            self.mint_per_block_rewards(&storage_cache.reward_token_id, Self::mint_rewards);
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
        &self,
        farm_token_amount: &BigUint<Self::Api>,
        token_attributes: &Self::AttributesType,
        storage_cache: &StorageCache<Self>,
    ) -> BigUint<Self::Api> {
        let token_rps = token_attributes.get_reward_per_share();
        if &storage_cache.reward_per_share > token_rps {
            let rps_diff = &storage_cache.reward_per_share - token_rps;
            farm_token_amount * &rps_diff / &storage_cache.division_safety_constant
        } else {
            BigUint::zero()
        }
    }

    fn create_enter_farm_initial_attributes(
        &self,
        farming_token_amount: BigUint<Self::Api>,
        current_reward_per_share: BigUint<Self::Api>,
    ) -> Self::AttributesType {
        let current_epoch = self.blockchain().get_block_epoch();
        let attributes = FarmTokenAttributes {
            reward_per_share: current_reward_per_share,
            entering_epoch: current_epoch,
            original_entering_epoch: current_epoch,
            initial_farming_amount: farming_token_amount.clone(),
            compounded_reward: BigUint::zero(),
            current_farm_amount: farming_token_amount,
        };

        transmute_or_panic::<Self::Api, _, _>(&attributes)
    }

    fn create_claim_rewards_initial_attributes(
        &self,
        first_token_attributes: Self::AttributesType,
        current_reward_per_share: BigUint<Self::Api>,
    ) -> Self::AttributesType {
        let initial_attributes: FarmTokenAttributes<Self::Api> =
            transmute_or_panic::<Self::Api, _, _>(&first_token_attributes);

        let net_current_farm_amount = initial_attributes.get_total_supply().clone();
        let new_attributes = FarmTokenAttributes {
            reward_per_share: current_reward_per_share,
            entering_epoch: initial_attributes.entering_epoch,
            original_entering_epoch: initial_attributes.original_entering_epoch,
            initial_farming_amount: initial_attributes.initial_farming_amount,
            compounded_reward: initial_attributes.compounded_reward,
            current_farm_amount: net_current_farm_amount,
        };

        transmute_or_panic::<Self::Api, _, _>(&new_attributes)
    }

    fn create_compound_rewards_initial_attributes(
        &self,
        first_token_attributes: Self::AttributesType,
        current_reward_per_share: BigUint<Self::Api>,
        reward: &BigUint<Self::Api>,
    ) -> Self::AttributesType {
        let initial_attributes: FarmTokenAttributes<Self::Api> =
            transmute_or_panic::<Self::Api, _, _>(&first_token_attributes);

        let current_epoch = self.blockchain().get_block_epoch();
        let new_pos_compounded_reward = initial_attributes.compounded_reward + reward;
        let new_pos_current_farm_amount = initial_attributes.current_farm_amount + reward;
        let new_attributes = FarmTokenAttributes {
            reward_per_share: current_reward_per_share,
            entering_epoch: current_epoch,
            original_entering_epoch: current_epoch,
            initial_farming_amount: initial_attributes.initial_farming_amount,
            compounded_reward: new_pos_compounded_reward,
            current_farm_amount: new_pos_current_farm_amount,
        };

        transmute_or_panic::<Self::Api, _, _>(&new_attributes)
    }
}

pub fn transmute_or_panic<M: ManagedTypeApi, FromType: 'static, ToType: 'static>(
    attr: &FromType,
) -> ToType {
    if TypeId::of::<FromType>() == TypeId::of::<ToType>() {
        unsafe { core::mem::transmute_copy::<FromType, ToType>(attr) }
    } else {
        M::error_api_impl()
            .signal_error(b"Must implement trait methods for custom attributes type");
    }
}
