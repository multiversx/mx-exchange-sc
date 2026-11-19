multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EventsModule: permissions_module::PermissionsModule {
    fn emit_whitelist_created_event(&self, whitelist_name: &ManagedBuffer) {
        self.whitelist_created_event(
            self.blockchain().get_caller(),
            self.blockchain().get_block_nonce(),
            self.blockchain().get_block_timestamp(),
            self.blockchain().get_block_epoch(),
            whitelist_name,
        );
    }

    fn emit_address_whitelisted_event(
        &self,
        whitelist_name: &ManagedBuffer,
        address: &ManagedAddress,
    ) {
        self.address_whitelisted_event(
            self.blockchain().get_caller(),
            self.blockchain().get_block_nonce(),
            self.blockchain().get_block_timestamp(),
            self.blockchain().get_block_epoch(),
            whitelist_name,
            address,
        );
    }

    fn emit_address_removed_event(&self, whitelist_name: &ManagedBuffer, address: &ManagedAddress) {
        self.address_removed_event(
            self.blockchain().get_caller(),
            self.blockchain().get_block_nonce(),
            self.blockchain().get_block_timestamp(),
            self.blockchain().get_block_epoch(),
            whitelist_name,
            address,
        );
    }

    #[event("whitelist_created")]
    fn whitelist_created_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] block_nonce: u64,
        #[indexed] block_timestamp: u64,
        #[indexed] block_epoch: u64,
        #[indexed] whitelist_name: &ManagedBuffer,
    );

    #[event("address_whitelisted")]
    fn address_whitelisted_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] block_nonce: u64,
        #[indexed] block_timestamp: u64,
        #[indexed] block_epoch: u64,
        #[indexed] whitelist_name: &ManagedBuffer,
        #[indexed] address: &ManagedAddress,
    );

    #[event("address_removed")]
    fn address_removed_event(
        &self,
        #[indexed] caller: ManagedAddress,
        #[indexed] block_nonce: u64,
        #[indexed] block_timestamp: u64,
        #[indexed] block_epoch: u64,
        #[indexed] whitelist_name: &ManagedBuffer,
        #[indexed] address: &ManagedAddress,
    );
}
