elrond_wasm::imports!();

use common_errors::ERROR_PERMISSIONS;

#[elrond_wasm::module]
pub trait WhitelistModule {
    #[only_owner]
    #[endpoint(addAddressToWhitelist)]
    fn add_address_to_whitelist(&self, address: ManagedAddress) {
        self.whitelisted(&address).set(&true);
    }

    #[only_owner]
    #[endpoint(removeAddressFromWhitelist)]
    fn remove_address_from_whitelist(&self, address: ManagedAddress) {
        self.whitelisted(&address).clear();
    }

    #[inline]
    fn is_whitelisted(&self, address: &ManagedAddress) -> bool {
        self.whitelisted(address).get()
    }

    fn require_whitelisted(&self, address: &ManagedAddress) {
        require!(self.is_whitelisted(address), ERROR_PERMISSIONS);
    }

    #[view(isWhitelisted)]
    #[storage_mapper("whitelisted")]
    fn whitelisted(&self, address: &ManagedAddress) -> SingleValueMapper<bool>;
}
