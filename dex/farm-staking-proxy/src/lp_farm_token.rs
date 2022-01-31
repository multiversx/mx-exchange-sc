elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::FarmTokenAttributes;

#[elrond_wasm::module]
pub trait LpFarmTokenModule: token_merge::TokenMergeModule {
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
        // TODO: Use new decode_or_panic function
        let attributes = token_data
            .decode_attributes::<FarmTokenAttributes<Self::Api>>()
            .unwrap();

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
