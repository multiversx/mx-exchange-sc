elrond_wasm::imports!();

use farm::ProxyTrait as _;

const DIVISION_SAFETY_CONST: u64 = 1_000_000_000_000_000_000;

#[elrond_wasm::module]
pub trait FarmDeployModule {
    #[endpoint(deployFarm)]
    fn deploy_farm(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        pair_contract_address: ManagedAddress,
    ) -> ManagedAddress {
        let owner_opt = OptionalValue::Some(self.blockchain().get_owner_address());
        let caller = self.blockchain().get_caller();
        let mut admins_list = MultiValueEncoded::new();
        admins_list.push(caller);

        let farm_template = self.farm_template_address().get();
        let code_metadata =
            CodeMetadata::PAYABLE_BY_SC & CodeMetadata::READABLE & CodeMetadata::UPGRADEABLE;
        let (new_farm_address, ()) = self
            .farm_deploy_proxy()
            .init(
                reward_token_id,
                farming_token_id,
                DIVISION_SAFETY_CONST,
                pair_contract_address,
                owner_opt,
                admins_list,
            )
            .deploy_from_source(&farm_template, code_metadata);

        new_farm_address
    }

    #[proxy]
    fn farm_deploy_proxy(&self) -> farm::Proxy<Self::Api>;

    #[storage_mapper("farmTemplateAddress")]
    fn farm_template_address(&self) -> SingleValueMapper<ManagedAddress>;
}
