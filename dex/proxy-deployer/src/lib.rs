#![no_std]

use deploy::ForcedDeployArgsType;

multiversx_sc::imports!();

pub mod deploy;
pub mod storage;
pub mod views;

#[multiversx_sc::contract]
pub trait ProxyDeployer: deploy::DeployModule + storage::StorageModule + views::ViewModule {
    /// Forced deploy args contain the index of the arg, and the argument itself.
    ///
    /// They must be provided in the order expected by the deployed SCs arguments order.
    ///
    /// Indexes start from 0
    #[init]
    fn init(
        &self,
        template_address: ManagedAddress,
        forced_deploy_args: ForcedDeployArgsType<Self::Api>,
    ) {
        require!(
            self.blockchain().is_smart_contract(&template_address),
            "Invalid farm template address"
        );

        self.overwrite_forced_deploy_args(forced_deploy_args);

        self.template_address().set(template_address);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
