multiversx_sc::imports!();
multiversx_sc::derive_imports!();

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
        max_farms_to_remove: usize,
    ) -> RemoveResult {
        // TODO: Remove admin from farm/farm_staking, then remove the farm/farm_staking from internal storage

        // TODO: Maybe also blacklist user?

        RemoveResult {
            any_farms_left: false,
        }
    }
}
