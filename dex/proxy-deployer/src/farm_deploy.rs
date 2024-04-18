use crate::farm_proxy;

multiversx_sc::imports!();

const DIVISION_SAFETY_CONST: u64 = 1_000_000_000_000_000_000;

#[multiversx_sc::module]
pub trait FarmDeployModule {
    #[endpoint(deployFarm)]
    fn deploy_farm(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        pair_contract_address: ManagedAddress,
    ) -> ManagedAddress {
        let owner = self.blockchain().get_owner_address();
        let caller = self.blockchain().get_caller();
        let mut admins_list = MultiValueEncoded::new();
        admins_list.push(caller.clone());

        let farm_template = self.farm_template_address().get();
        let code_metadata =
            CodeMetadata::PAYABLE_BY_SC | CodeMetadata::READABLE | CodeMetadata::UPGRADEABLE;

        let new_farm_address = self
            .tx()
            .typed(farm_proxy::FarmProxy)
            .init(
                reward_token_id,
                farming_token_id,
                DIVISION_SAFETY_CONST,
                pair_contract_address,
                owner,
                admins_list,
            )
            .from_source(farm_template)
            .code_metadata(code_metadata)
            .returns(ReturnsNewManagedAddress)
            .sync_call();

        self.deployer_farm_addresses(&caller)
            .update(|farm_addresses| {
                farm_addresses.push(new_farm_address.clone());
            });
        self.deployers_list().insert(caller);

        new_farm_address
    }

    #[only_owner]
    #[endpoint(callFarmEndpoint)]
    fn call_farm_endpoint(
        &self,
        farm_address: ManagedAddress,
        function_name: ManagedBuffer,
        args: MultiValueEncoded<ManagedBuffer>,
    ) {
        let gas_left = self.blockchain().get_gas_left();

        self.tx()
            .to(&farm_address)
            .raw_call(function_name)
            .gas(gas_left)
            .arguments_raw(args.to_arg_buffer())
            .sync_call();
    }

    #[view(getAllDeployedFarms)]
    fn get_all_deployed_farms(&self) -> ManagedVec<ManagedAddress> {
        let mut all_farm_addresses = ManagedVec::new();
        for deployer in self.deployers_list().iter() {
            all_farm_addresses.append_vec(self.deployer_farm_addresses(&deployer).get());
        }
        all_farm_addresses
    }

    #[storage_mapper("farmTemplateAddress")]
    fn farm_template_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("deployersList")]
    fn deployers_list(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[view(getDeployerFarmAddresses)]
    #[storage_mapper("deployerFarmAddresses")]
    fn deployer_farm_addresses(
        &self,
        deployer_address: &ManagedAddress,
    ) -> SingleValueMapper<ManagedVec<ManagedAddress>>;
}
