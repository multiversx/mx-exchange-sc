#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::{FarmToken, Nonce, PaymentsVec};
use pausable::State;

pub const DEFAULT_NFT_DEPOSIT_MAX_LEN: usize = 10;
pub const DEFAULT_FARM_POSITION_MIGRATION_NONCE: u64 = 1;

#[multiversx_sc::module]
pub trait ConfigModule: pausable::PausableModule + permissions_module::PermissionsModule {
    #[inline]
    fn is_active(&self) -> bool {
        let state = self.state().get();
        state == State::Active
    }

    fn is_old_farm_position(&self, token_nonce: Nonce) -> bool {
        let farm_position_migration_nonce = self.farm_position_migration_nonce().get();
        token_nonce > 0 && token_nonce < farm_position_migration_nonce
    }

    fn try_set_farm_position_migration_nonce(
        &self,
        farm_token_mapper: NonFungibleTokenMapper<Self::Api>,
    ) {
        if !self.farm_position_migration_nonce().is_empty() {
            return;
        }

        let migration_farm_token_nonce = if farm_token_mapper.get_token_state().is_set() {
            let token_identifier = farm_token_mapper.get_token_id_ref();
            let current_nonce = self
                .blockchain()
                .get_current_esdt_nft_nonce(&self.blockchain().get_sc_address(), token_identifier);
            current_nonce + DEFAULT_FARM_POSITION_MIGRATION_NONCE
        } else {
            DEFAULT_FARM_POSITION_MIGRATION_NONCE
        };

        self.farm_position_migration_nonce()
            .set(migration_farm_token_nonce);
    }

    fn check_and_update_user_farm_position<T: FarmToken<Self::Api> + TopDecode>(
        &self,
        user: &ManagedAddress,
        farm_positions: &PaymentsVec<Self::Api>,
        farm_token_mapper: &NonFungibleTokenMapper<Self::Api>,
    ) {
        for farm_position in farm_positions {
            farm_token_mapper.require_same_token(&farm_position.token_identifier);

            if self.is_old_farm_position(farm_position.token_nonce) {
                continue;
            }

            let token_attributes: T =
                farm_token_mapper.get_token_attributes(farm_position.token_nonce);

            if &token_attributes.get_original_owner() != user {
                self.decrease_user_farm_position::<T>(&farm_position, farm_token_mapper);
                self.increase_user_farm_position(user, &farm_position.amount);
            }
        }
    }

    #[inline]
    fn increase_user_farm_position(
        &self,
        user: &ManagedAddress,
        increase_farm_position_amount: &BigUint,
    ) {
        self.user_total_farm_position(user)
            .update(|total_farm_position| *total_farm_position += increase_farm_position_amount);
    }

    fn decrease_user_farm_position<T: FarmToken<Self::Api> + TopDecode>(
        &self,
        farm_position: &EsdtTokenPayment,
        farm_token_mapper: &NonFungibleTokenMapper<Self::Api>,
    ) {
        if self.is_old_farm_position(farm_position.token_nonce) {
            return;
        }

        let token_attributes: T = farm_token_mapper.get_token_attributes(farm_position.token_nonce);
        let user_total_farm_position_mapper =
            self.user_total_farm_position(&token_attributes.get_original_owner());
        let mut user_total_farm_position = user_total_farm_position_mapper.get();

        if user_total_farm_position > farm_position.amount {
            user_total_farm_position -= &farm_position.amount;
            user_total_farm_position_mapper.set(user_total_farm_position);
        } else {
            user_total_farm_position_mapper.clear();
        }
    }

    #[view(getFarmingTokenId)]
    #[storage_mapper("farming_token_id")]
    fn farming_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getRewardTokenId)]
    #[storage_mapper("reward_token_id")]
    fn reward_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getPerBlockRewardAmount)]
    #[storage_mapper("per_block_reward_amount")]
    fn per_block_reward_amount(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("produce_rewards_enabled")]
    fn produce_rewards_enabled(&self) -> SingleValueMapper<bool>;

    #[view(getLastRewardBlockNonce)]
    #[storage_mapper("last_reward_block_nonce")]
    fn last_reward_block_nonce(&self) -> SingleValueMapper<Nonce>;

    #[view(getDivisionSafetyConstant)]
    #[storage_mapper("division_safety_constant")]
    fn division_safety_constant(&self) -> SingleValueMapper<BigUint>;

    #[view(getUserTotalFarmPosition)]
    #[storage_mapper("userTotalFarmPosition")]
    fn user_total_farm_position(&self, user: &ManagedAddress) -> SingleValueMapper<BigUint>;

    // Update for this storage disabled for this version of the exchange
    #[view(getAllowExternalClaim)]
    #[storage_mapper("allowExternalClaim")]
    fn allow_external_claim(&self, user: &ManagedAddress) -> SingleValueMapper<bool>;

    #[view(getFarmPositionMigrationNonce)]
    #[storage_mapper("farm_position_migration_nonce")]
    fn farm_position_migration_nonce(&self) -> SingleValueMapper<Nonce>;
}
