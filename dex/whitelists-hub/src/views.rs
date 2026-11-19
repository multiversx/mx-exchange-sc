multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ViewsModule:
    permissions_module::PermissionsModule
    + crate::whitelist::WhitelistModule
    + crate::events::EventsModule
{
    /// Check if an address is in a specific whitelist
    #[view(isWhitelisted)]
    fn is_whitelisted(&self, whitelist_name: &ManagedBuffer, address: &ManagedAddress) -> bool {
        if !self.whitelist_exists(whitelist_name) {
            return false;
        }

        self.whitelist_addresses(whitelist_name).contains(address)
    }

    /// Get all addresses in a whitelist
    #[view(getWhitelistAddresses)]
    fn get_whitelist_addresses(
        &self,
        whitelist_name: &ManagedBuffer,
    ) -> MultiValueEncoded<ManagedAddress> {
        require!(
            self.whitelist_exists(whitelist_name),
            "Whitelist does not exist"
        );

        let mut result = MultiValueEncoded::new();
        for address in self.whitelist_addresses(whitelist_name).iter() {
            result.push(address);
        }

        result
    }

    /// Get all whitelist names
    #[view(getAllWhitelists)]
    fn get_all_whitelists(&self) -> MultiValueEncoded<ManagedBuffer> {
        let mut result = MultiValueEncoded::new();
        for name in self.whitelist_registry().iter() {
            result.push(name);
        }

        result
    }

    /// Count addresses in a whitelist
    #[view(countAddressesInWhitelist)]
    fn count_addresses_in_whitelist(&self, whitelist_name: &ManagedBuffer) -> usize {
        if !self.whitelist_exists(whitelist_name) {
            return 0;
        }

        self.whitelist_addresses(whitelist_name).len()
    }

    /// Get all whitelist info - names and counts
    #[view(getAllWhitelistInfo)]
    fn get_all_whitelist_info(&self) -> MultiValueEncoded<MultiValue2<ManagedBuffer, usize>> {
        let mut result = MultiValueEncoded::new();

        for name in self.whitelist_registry().iter() {
            let count = self.whitelist_addresses(&name).len();
            result.push((name, count).into());
        }

        result
    }

    /// Check in a batch if multiple addresses are in a whitelist
    #[view(batchIsWhitelisted)]
    fn batch_is_whitelisted(
        &self,
        whitelist_name: &ManagedBuffer,
        addresses: MultiValueEncoded<ManagedAddress>,
    ) -> MultiValueEncoded<MultiValue2<ManagedAddress, bool>> {
        let mut result = MultiValueEncoded::new();

        if !self.whitelist_exists(whitelist_name) {
            for address in addresses {
                result.push((address, false).into());
            }
            return result;
        }

        let addresses_mapper = self.whitelist_addresses(whitelist_name);
        for address in addresses {
            let is_whitelisted = addresses_mapper.contains(&address);
            result.push((address, is_whitelisted).into());
        }

        result
    }
}
