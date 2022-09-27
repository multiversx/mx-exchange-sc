#![no_std]

use common_errors::ERROR_PARAMETERS;

elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait WhitelistModule {
    #[only_owner]
    #[endpoint]
    fn add_address_to_whitelist(&self, address: ManagedAddress) {
        let addresses_mapper = self.whitelist_addresses();
        require!(!addresses_mapper.contains(&address), ERROR_PARAMETERS);
        self.whitelist_addresses().add(&address);
    }

    #[only_owner]
    #[endpoint]
    fn remove_address_from_whitelist(&self, address: ManagedAddress) {
        let addresses_mapper = self.whitelist_addresses();
        require!(addresses_mapper.contains(&address), ERROR_PARAMETERS);
        self.whitelist_addresses().remove(&address);
    }

    fn is_address_whitelisted(&self, address: &ManagedAddress) -> bool {
        self.whitelist_addresses().contains(address)
    }

    #[storage_mapper("whitelistAddresses")]
    fn whitelist_addresses(&self) -> WhitelistMapper<Self::Api, ManagedAddress>;
}
