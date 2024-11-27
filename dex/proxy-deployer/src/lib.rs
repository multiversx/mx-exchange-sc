#![no_std]

use storage::DeployerType;

multiversx_sc::imports!();

pub mod deploy;
pub mod storage;
pub mod views;

#[multiversx_sc::contract]
pub trait ProxyDeployer: deploy::DeployModule + storage::StorageModule + views::ViewModule {
    #[init]
    fn init(&self, template_address: ManagedAddress, deployer_type: DeployerType) {
        require!(
            self.blockchain().is_smart_contract(&template_address),
            "Invalid farm template address"
        );
        require!(deployer_type != DeployerType::None, "Invalid deployer type");

        self.template_address().set(template_address);
        self.deployer_type().set(deployer_type);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
