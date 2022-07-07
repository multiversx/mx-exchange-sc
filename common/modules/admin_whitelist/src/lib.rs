elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait AdminWhitelistModule {
    #[only_owner]
    #[endpoint(addAdmins)]
    fn add_admins(&self, admins: MultiValueEncoded<ManagedAddress>) {
        let mut whitelist = self.admins();
        for admin in admins {
            let _ = whitelist.insert(admin);
        }
    }

    #[only_owner]
    #[endpoint(removeAdmins)]
    fn remove_admins(&self, admins: MultiValueEncoded<ManagedAddress>) {
        let mut whitelist = self.admins();
        for admin in admins {
            let _ = whitelist.swap_remove(&admin);
        }
    }

    fn require_caller_is_admin(&self) {
        let caller = self.blockchain().get_caller();
        require!(self.admins().contains(&caller), "Caller is not an admin");
    }

    fn require_caller_is_owner_or_admin(&self) {
        let caller = self.blockchain().get_caller();
        let owner = self.blockchain().get_owner_address();
        require!(
            caller == owner || self.admins().contains(&caller),
            "Caller is not owner or an admin"
        );
    }

    #[view(getAdmins)]
    #[storage_mapper("admins")]
    fn admins(&self) -> UnorderedSetMapper<ManagedAddress>;
}
