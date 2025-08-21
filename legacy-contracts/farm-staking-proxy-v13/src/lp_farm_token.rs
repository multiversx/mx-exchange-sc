multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(
    ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, PartialEq, Debug,
)]
pub struct FarmTokenAttributes<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub original_entering_epoch: u64,
    pub entering_epoch: u64,
    pub initial_farming_amount: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
}

#[multiversx_sc::module]
pub trait LpFarmTokenModule: token_merge_helper::TokenMergeHelperModule {
    fn get_lp_tokens_in_farm_position(
        &self,
        farm_token_nonce: u64,
        farm_token_amount: &BigUint,
    ) -> BigUint {
        let own_sc_address = self.blockchain().get_sc_address();
        let lp_farm_token_id = self.lp_farm_token_id().get();
        let token_data = self.blockchain().get_esdt_token_data(
            &own_sc_address,
            &lp_farm_token_id,
            farm_token_nonce,
        );
        let attributes = token_data.decode_attributes::<FarmTokenAttributes<Self::Api>>();

        self.rule_of_three_non_zero_result(
            farm_token_amount,
            &attributes.current_farm_amount,
            &attributes.initial_farming_amount,
        )
    }

    #[view(getLpFarmTokenId)]
    #[storage_mapper("lpFarmTokenId")]
    fn lp_farm_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
