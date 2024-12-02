#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::contract]
pub trait PermissionsHub {
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[endpoint]
    fn whitelist(&self, addresses_to_whitelist: MultiValueEncoded<ManagedAddress>) {
        let caller = self.blockchain().get_caller();
        for address_to_whitelist in addresses_to_whitelist.into_iter() {
            require!(
                self.user_whitelisted_addresses(&caller)
                    .insert(address_to_whitelist),
                "Address is already whitelisted"
            );
        }
    }

    #[endpoint(removeWhitelist)]
    fn remove_whitelist(&self, addresses_to_remove: MultiValueEncoded<ManagedAddress>) {
        let caller = self.blockchain().get_caller();
        for address_to_remove in addresses_to_remove.into_iter() {
            require!(
                self.user_whitelisted_addresses(&caller)
                    .swap_remove(&address_to_remove),
                "Address is not whitelisted"
            );
        }
    }

    #[only_owner]
    #[endpoint(blacklist)]
    fn blacklist(&self, address_to_blacklist: ManagedAddress) {
        self.blacklisted_addresses().insert(address_to_blacklist);
    }

    #[only_owner]
    #[endpoint(removeBlacklist)]
    fn remove_blacklist(&self, address_to_remove: ManagedAddress) {
        self.blacklisted_addresses().swap_remove(&address_to_remove);
    }

    #[view(isWhitelisted)]
    fn is_whitelisted(&self, user: &ManagedAddress, address_to_check: &ManagedAddress) -> bool {
        !self.blacklisted_addresses().contains(address_to_check)
            && self
                .user_whitelisted_addresses(user)
                .contains(address_to_check)
    }

    #[storage_mapper("whitelistedAddresses")]
    fn user_whitelisted_addresses(
        &self,
        user: &ManagedAddress,
    ) -> UnorderedSetMapper<ManagedAddress>;

    #[view(getBlacklistedAddresses)]
    #[storage_mapper("blacklistedAddresses")]
    fn blacklisted_addresses(&self) -> UnorderedSetMapper<ManagedAddress>;
}
