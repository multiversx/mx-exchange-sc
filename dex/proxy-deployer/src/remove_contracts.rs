multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use permissions_module::ProxyTrait as _;

#[derive(TypeAbi, TopEncode)]
pub struct RemoveResult {
    pub any_farms_left: bool,
}

#[multiversx_sc::module]
pub trait RemoveContractsModule: crate::storage::StorageModule {
    #[only_owner]
    #[endpoint(removeAllByDeployer)]
    fn remove_all_by_deployer(
        &self,
        deployer_address: ManagedAddress,
        max_to_remove: usize,
    ) -> RemoveResult {
        let id_mapper = self.address_id();
        let deployer_id = id_mapper.get_id_non_zero(&deployer_address);
        self.user_blacklist().add(&deployer_id);

        let mut contracts_mapper = self.contracts_by_address(deployer_id);

        let total_contracts = contracts_mapper.len();
        let to_remove = core::cmp::min(total_contracts, max_to_remove);
        for _ in 0..to_remove {
            let contract_id = contracts_mapper.get_by_index(1);
            let contract_address = self.get_by_id(&id_mapper, contract_id);
            let _ = contracts_mapper.swap_remove(&contract_id);

            self.remove_admin(contract_address, deployer_address.clone());
            self.remove_contract(contract_id);
        }

        RemoveResult {
            any_farms_left: total_contracts > max_to_remove,
        }
    }

    #[only_owner]
    #[endpoint(removeSingleContract)]
    fn remove_single_contract(&self, contract: ManagedAddress) {
        let id_mapper = self.address_id();
        let contract_id = id_mapper.get_id_non_zero(&contract);
        let deployer_id = self.contract_owner(contract_id).get();
        require!(deployer_id != 0, "Contract already removed");

        let _ = self
            .contracts_by_address(deployer_id)
            .swap_remove(&contract_id);

        let deployer_address = self.get_by_id(&id_mapper, deployer_id);
        self.remove_admin(contract, deployer_address);
        self.remove_contract(contract_id);
    }

    fn remove_admin(&self, contract: ManagedAddress, user: ManagedAddress) {
        self.remove_user_proxy(contract)
            .remove_admin_endpoint(user)
            .execute_on_dest_context()
    }

    fn remove_contract(&self, contract_id: AddressId) {
        let _ = self.all_deployed_contracts().swap_remove(&contract_id);
        let token_for_address = self.token_for_address(contract_id).take();
        let _ = self.all_used_tokens().swap_remove(&token_for_address);
        self.address_for_token(&token_for_address).clear();
        self.contract_owner(contract_id).clear();
    }

    // For now, both farm and farm_staking use the same internal permissions module.
    //
    // Create two separate proxies if this if this ever changes.
    #[proxy]
    fn remove_user_proxy(&self, sc_address: ManagedAddress) -> farm_staking::Proxy<Self::Api>;
}
