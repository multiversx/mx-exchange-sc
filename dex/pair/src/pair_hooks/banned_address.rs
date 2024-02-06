multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait BannedAddressModule: permissions_module::PermissionsModule {
    #[endpoint(addBannedAddress)]
    fn add_banned_address(&self, addresses: MultiValueEncoded<ManagedAddress>) {
        self.require_caller_has_owner_or_admin_permissions();

        let mapper = self.banned_addresses();
        for address in addresses {
            mapper.add(&address);
        }
    }

    #[endpoint(removeBannedAddress)]
    fn remove_banned_address(&self, addresses: MultiValueEncoded<ManagedAddress>) {
        self.require_caller_has_owner_or_admin_permissions();

        let mapper = self.banned_addresses();
        for address in addresses {
            mapper.remove(&address);
        }
    }

    fn require_not_banned_address(&self, address: &ManagedAddress) {
        require!(
            !self.banned_addresses().contains(address),
            "Cannot add hook for this address"
        );
    }

    #[storage_mapper("bannedAddresses")]
    fn banned_addresses(&self) -> WhitelistMapper<ManagedAddress>;
}
