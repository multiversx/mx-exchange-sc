#![no_std]

use farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactors;
use storage::DeployerType;

multiversx_sc::imports!();

pub mod deploy;
pub mod remove_contracts;
pub mod set_contract_active;
pub mod storage;
pub mod views;

#[multiversx_sc::contract]
pub trait ProxyDeployer:
    deploy::DeployModule
    + set_contract_active::SetContractActiveModule
    + remove_contracts::RemoveContractsModule
    + storage::StorageModule
    + views::ViewModule
{
    #[init]
    fn init(
        &self,
        template_address: ManagedAddress,
        deployer_type: DeployerType,
        timestamp_oracle_address: ManagedAddress,
        boosted_yields_factors: BoostedYieldsFactors<Self::Api>,
    ) {
        require!(
            self.blockchain().is_smart_contract(&template_address),
            "Invalid farm template address"
        );
        require!(deployer_type != DeployerType::None, "Invalid deployer type");
        require!(
            self.blockchain()
                .is_smart_contract(&timestamp_oracle_address),
            "Invalid timestamp oracle address"
        );

        self.template_address().set(template_address);
        self.deployer_type().set(deployer_type);
        self.timestamp_oracle_address()
            .set(timestamp_oracle_address);
        self.boosted_yields_factors().set(boosted_yields_factors);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
