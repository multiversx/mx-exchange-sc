#![no_std]

use common_errors::ERROR_PARAMETERS;

elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait SCWhitelistModule {
    #[only_owner]
    #[endpoint(addSCAddressToWhitelist)]
    fn add_sc_address_to_whitelist(&self, address: ManagedAddress) {
        let addresses_mapper = self.sc_whitelist_addresses();
        require!(!addresses_mapper.contains(&address), ERROR_PARAMETERS);
        self.sc_whitelist_addresses().add(&address);
    }

    #[only_owner]
    #[endpoint(removeSCAddressFromWhitelist)]
    fn remove_sc_address_from_whitelist(&self, address: ManagedAddress) {
        let addresses_mapper = self.sc_whitelist_addresses();
        require!(addresses_mapper.contains(&address), ERROR_PARAMETERS);
        self.sc_whitelist_addresses().remove(&address);
    }

    #[view(isSCAddressWhitelisted)]
    fn is_sc_address_whitelisted(&self, address: ManagedAddress) -> bool {
        self.sc_whitelist_addresses().contains(&address)
    }

    #[inline]
    fn require_sc_address_whitelisted(&self, address: &ManagedAddress) {
        self.sc_whitelist_addresses().require_whitelisted(address);
    }

    #[storage_mapper("scWhitelistAddresses")]
    fn sc_whitelist_addresses(&self) -> WhitelistMapper<Self::Api, ManagedAddress>;
}
