multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait WhitelistModule:
    permissions_module::PermissionsModule + crate::events::EventsModule
{
    #[endpoint(createWhitelist)]
    fn create_whitelist(&self, whitelist_name: ManagedBuffer) {
        self.require_caller_has_admin_permissions();
        require!(!whitelist_name.is_empty(), "Whitelist name cannot be empty");
        require!(
            !self.whitelist_exists(&whitelist_name),
            "Whitelist already exists"
        );

        self.verify_whitelist_name_safety(&whitelist_name);

        self.whitelist_registry().insert(whitelist_name.clone());

        self.emit_whitelist_created_event(&whitelist_name);
    }

    #[endpoint(addToWhitelist)]
    fn add_to_whitelist(
        &self,
        whitelist_name: ManagedBuffer,
        addresses: MultiValueEncoded<ManagedAddress>,
    ) {
        self.require_caller_has_admin_permissions();
        require!(
            self.whitelist_exists(&whitelist_name),
            "Whitelist does not exist"
        );

        let mut addresses_mapper = self.whitelist_addresses(&whitelist_name);
        for address in addresses {
            require!(!address.is_zero(), "Address cannot be zero");
            if addresses_mapper.insert(address.clone()) {
                self.emit_address_whitelisted_event(&whitelist_name, &address);
            }
        }
    }

    #[endpoint(removeFromWhitelist)]
    fn remove_from_whitelist(
        &self,
        whitelist_name: ManagedBuffer,
        addresses: MultiValueEncoded<ManagedAddress>,
    ) {
        self.require_caller_has_admin_permissions();
        require!(
            self.whitelist_exists(&whitelist_name),
            "Whitelist does not exist"
        );

        let mut addresses_mapper = self.whitelist_addresses(&whitelist_name);
        for address in addresses {
            if addresses_mapper.swap_remove(&address) {
                self.emit_address_removed_event(&whitelist_name, &address);
            }
        }
    }

    fn whitelist_exists(&self, whitelist_name: &ManagedBuffer) -> bool {
        self.whitelist_registry().contains(whitelist_name)
    }

    // Helper function to verify that a whitelist name doesn't create storage key conflicts
    fn verify_whitelist_name_safety(&self, new_name: &ManagedBuffer) {
        for existing_name in self.whitelist_registry().iter() {
            let new_name_len = new_name.len();
            let existing_name_len = existing_name.len();

            // Check if new name is a prefix of existing name
            if new_name_len <= existing_name_len {
                let existing_prefix = existing_name.copy_slice(0, new_name_len);
                if let Some(prefix) = existing_prefix {
                    require!(
                        &prefix != new_name,
                        "New whitelist name is a prefix of an existing whitelist"
                    );
                }
            }

            // Check if existing name is a prefix of new name
            if existing_name_len <= new_name_len {
                let new_prefix = new_name.copy_slice(0, existing_name_len);
                if let Some(prefix) = new_prefix {
                    require!(
                        prefix != existing_name,
                        "An existing whitelist name is a prefix of the new whitelist name"
                    );
                }
            }
        }
    }

    /// Registry of all created whitelists
    #[storage_mapper("whitelist_registry")]
    fn whitelist_registry(&self) -> UnorderedSetMapper<ManagedBuffer>;

    /// Mapper for each whitelist's addresses
    /// Uses a dynamic key based on the whitelist name
    #[storage_mapper("whitelist_addresses")]
    fn whitelist_addresses(
        &self,
        whitelist_name: &ManagedBuffer,
    ) -> UnorderedSetMapper<ManagedAddress>;
}
