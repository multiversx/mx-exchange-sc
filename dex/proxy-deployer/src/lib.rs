#![no_std]

multiversx_sc::imports!();

pub mod farm_deploy;

#[multiversx_sc::contract]
pub trait ProxyDeployer: farm_deploy::FarmDeployModule {
    #[init]
    fn init(&self, farm_template_address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&farm_template_address),
            "Invalid farm template address"
        );

        self.farm_template_address().set(&farm_template_address);
    }
}
