elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::FarmTokenAttributes;
use fixed_supply_token::FixedSupplyToken;

#[elrond_wasm::module]
pub trait LpFarmTokenModule: utils::UtilsModule {
    fn get_lp_tokens_in_farm_position(
        &self,
        farm_token_nonce: u64,
        farm_token_amount: &BigUint,
    ) -> BigUint {
        let lp_farm_token_id = self.lp_farm_token_id().get();
        let attributes = self
            .get_token_attributes::<FarmTokenAttributes<Self::Api>>(
                &lp_farm_token_id,
                farm_token_nonce,
            )
            .into_part(farm_token_amount);

        attributes.current_farm_amount
    }

    #[view(getLpFarmTokenId)]
    #[storage_mapper("lpFarmTokenId")]
    fn lp_farm_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
