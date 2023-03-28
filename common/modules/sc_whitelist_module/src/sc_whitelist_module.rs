#![no_std]

use common_errors::ERROR_PARAMETERS;

multiversx_sc::imports!();

#[multiversx_sc::module]
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

    fn get_orig_caller_from_opt(
        &self,
        caller: &ManagedAddress,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> ManagedAddress {
        match opt_orig_caller {
            OptionalValue::Some(opt_caller) => {
                self.require_sc_address_whitelisted(caller);
                opt_caller
            }
            OptionalValue::None => caller.clone(),
        }
    }

    #[storage_mapper("scWhitelistAddresses")]
    fn sc_whitelist_addresses(&self) -> WhitelistMapper<ManagedAddress>;
}
