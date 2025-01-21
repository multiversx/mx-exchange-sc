#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait PermissionsHubModule {
    fn require_user_whitelisted(&self, user: &ManagedAddress, authorized_address: &ManagedAddress) {
        let permissions_hub_address = self.permissions_hub_address().get();
        let is_whitelisted: bool = self
            .permissions_hub_proxy(permissions_hub_address)
            .is_whitelisted(user, authorized_address)
            .execute_on_dest_context();

        require!(
            is_whitelisted,
            "Caller is not whitelisted by the user or is blacklisted"
        );
    }

    #[only_owner]
    #[endpoint(setPermissionsHubAddress)]
    fn set_permissions_hub_address(&self, address: ManagedAddress) {
        self.permissions_hub_address().set(&address);
    }

    #[proxy]
    fn permissions_hub_proxy(
        &self,
        sc_address: ManagedAddress,
    ) -> permissions_hub::Proxy<Self::Api>;

    #[storage_mapper("permissionsHubAddress")]
    fn permissions_hub_address(&self) -> SingleValueMapper<ManagedAddress>;
}
