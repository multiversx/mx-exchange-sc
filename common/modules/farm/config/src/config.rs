#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Nonce;
use pausable::State;

pub const DEFAULT_NFT_DEPOSIT_MAX_LEN: usize = 10;

#[derive(
    ManagedVecItem,
    TopEncode,
    TopDecode,
    NestedEncode,
    NestedDecode,
    TypeAbi,
    Clone,
    PartialEq,
    Debug,
)]
pub struct UserTotalFarmPositionStruct<M: ManagedTypeApi> {
    pub total_farm_position: BigUint<M>,
    pub allow_external_claim_boosted_rewards: bool,
}

#[multiversx_sc::module]
pub trait ConfigModule: pausable::PausableModule + permissions_module::PermissionsModule {
    #[inline]
    fn is_active(&self) -> bool {
        let state = self.state().get();
        state == State::Active
    }

    fn get_user_total_farm_position_struct(
        &self,
        user: &ManagedAddress,
    ) -> UserTotalFarmPositionStruct<Self::Api> {
        self.user_total_farm_position(user)
            .set_if_empty(UserTotalFarmPositionStruct {
                total_farm_position: BigUint::zero(),
                allow_external_claim_boosted_rewards: false,
            });

        self.user_total_farm_position(user).get()
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
    fn user_total_farm_position(
        &self,
        user: &ManagedAddress,
    ) -> SingleValueMapper<UserTotalFarmPositionStruct<Self::Api>>;
}
