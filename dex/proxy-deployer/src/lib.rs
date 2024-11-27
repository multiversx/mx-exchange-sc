#![no_std]

use storage::DeployerType;

multiversx_sc::imports!();

pub mod deploy;
pub mod remove_contracts;
pub mod storage;
pub mod views;

#[multiversx_sc::contract]
pub trait ProxyDeployer:
    deploy::DeployModule
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
    }

    #[upgrade]
    fn upgrade(&self) {}
}
