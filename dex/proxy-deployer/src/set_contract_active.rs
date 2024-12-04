use common_structs::Percent;
use farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactors;
use farm_boosted_yields::boosted_yields_factors::ProxyTrait as _;
use farm_staking::custom_rewards::ProxyTrait as _;
use pausable::ProxyTrait as _;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait SetContractActiveModule:
    crate::storage::StorageModule + crate::remove_contracts::RemoveContractsModule
{
    /// Boosted yields percent must be >= 0 (0%) and <= 10_000 (100%)
    ///
    /// Only callable by contract deployer
    ///
    /// Calling this endpoint multiple times is the same as calling the specific endpoints in farm by the deployer
    ///
    /// NOTE: Must issue farm token first!
    #[endpoint(setContractActive)]
    fn set_contract_active(
        &self,
        contract: ManagedAddress,
        rewards_per_block: BigUint,
        boosted_yields_percent: Percent,
    ) {
        let id_mapper = self.address_id();
        let contract_id = id_mapper.get_id_non_zero(&contract);

        let caller = self.blockchain().get_caller();
        let caller_id = id_mapper.get_id_non_zero(&caller);
        let owner_id = self.contract_owner(contract_id).get();
        require!(
            caller_id == owner_id,
            "Only contract owner may call this endpoint"
        );

        let boosted_yields_factors = self.boosted_yields_factors().get();
        self.set_rewards_per_block(contract.clone(), rewards_per_block);
        self.set_boosted_yields_factors(contract.clone(), boosted_yields_factors);
        self.set_boosted_yields_percent(contract.clone(), boosted_yields_percent);
        self.unpause_contract(contract.clone());

        // Contract was only added as admin so we don't have to change all the permissions around
        let own_sc_address = self.blockchain().get_sc_address();
        self.remove_admin(contract, own_sc_address);
    }

    fn set_rewards_per_block(&self, contract: ManagedAddress, rewards_per_block: BigUint) {
        self.set_contract_active_proxy(contract)
            .set_per_block_rewards(rewards_per_block)
            .execute_on_dest_context()
    }

    fn set_boosted_yields_factors(
        &self,
        contract: ManagedAddress,
        factors: BoostedYieldsFactors<Self::Api>,
    ) {
        self.set_contract_active_proxy(contract)
            .set_boosted_yields_factors(
                factors.max_rewards_factor,
                factors.user_rewards_energy_const,
                factors.user_rewards_farm_const,
                factors.min_energy_amount,
                factors.min_farm_amount,
            )
            .execute_on_dest_context()
    }

    fn set_boosted_yields_percent(
        &self,
        contract: ManagedAddress,
        boosted_yields_percent: Percent,
    ) {
        self.set_contract_active_proxy(contract)
            .set_boosted_yields_rewards_percentage(boosted_yields_percent)
            .execute_on_dest_context()
    }

    fn unpause_contract(&self, contract: ManagedAddress) {
        self.set_contract_active_proxy(contract)
            .resume()
            .execute_on_dest_context()
    }

    #[proxy]
    fn set_contract_active_proxy(
        &self,
        sc_address: ManagedAddress,
    ) -> farm_staking::Proxy<Self::Api>;
}
