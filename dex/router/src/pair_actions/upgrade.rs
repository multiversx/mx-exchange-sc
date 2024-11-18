multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait UpgradeModule:
    crate::config::ConfigModule
    + pair::read_pair_storage::ReadPairStorageModule
    + super::create::CreateModule
    + crate::temp_owner::TempOwnerModule
    + crate::events::EventsModule
    + crate::state::StateModule
    + crate::views::ViewsModule
{
    #[only_owner]
    #[endpoint(upgradePair)]
    fn upgrade_pair_endpoint(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) {
        self.require_active();
        require!(first_token_id != second_token_id, "Identical tokens");
        require!(
            first_token_id.is_valid_esdt_identifier(),
            "First Token ID is not a valid esdt token ID"
        );
        require!(
            second_token_id.is_valid_esdt_identifier(),
            "Second Token ID is not a valid esdt token ID"
        );
        let pair_address = self.get_pair(first_token_id.clone(), second_token_id.clone());
        require!(!pair_address.is_zero(), "Pair does not exists");

        self.upgrade_pair(pair_address);
    }

    fn upgrade_pair(&self, pair_address: ManagedAddress) {
        let pair_template_address = self.pair_template_address().get();
        let code_metadata = self.get_default_code_metadata();
        self.tx()
            .to(pair_address)
            .raw_upgrade()
            .from_source(pair_template_address)
            .code_metadata(code_metadata)
            .upgrade_async_call_and_exit();
    }
}
