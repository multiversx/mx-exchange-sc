multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait UpgradeModule:
    crate::config::ConfigModule + pair::read_pair_storage::ReadPairStorageModule
{
    fn upgrade_pair(&self, pair_address: ManagedAddress) {
        let pair_template_address = self.pair_template_address().get();
        let code_metadata =
            CodeMetadata::UPGRADEABLE | CodeMetadata::READABLE | CodeMetadata::PAYABLE_BY_SC;
        self.tx()
            .to(pair_address)
            .raw_upgrade()
            .from_source(pair_template_address)
            .code_metadata(code_metadata)
            .upgrade_async_call_and_exit();
    }
}
