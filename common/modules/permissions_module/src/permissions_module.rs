#![no_std]

mod permissions;

use common_errors::ERROR_PERMISSION_DENIED;

pub use permissions::Permissions;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait PermissionsModule {
    #[endpoint(addAdmin)]
    fn add_admin_endpoint(&self, address: ManagedAddress) {
        self.require_caller_has_owner_permissions();
        self.add_permissions(address, Permissions::ADMIN);
    }

    #[endpoint(removeAdmin)]
    fn remove_admin_endpoint(&self, address: ManagedAddress) {
        self.require_caller_has_owner_permissions();
        self.remove_permissions(address, Permissions::ADMIN);
    }

    #[only_owner]
    #[endpoint(updateOwnerOrAdmin)]
    fn update_owner_or_admin_endpoint(&self, previous_owner: ManagedAddress) {
        let caller = self.blockchain().get_caller();
        let previous_owner_permissions = self.permissions(previous_owner.clone()).get();

        self.permissions(previous_owner).clear();
        self.permissions(caller).set(previous_owner_permissions);
    }

    fn set_permissions(&self, address: ManagedAddress, permissions: Permissions) {
        self.permissions(address).set(permissions);
    }

    fn add_permissions(&self, address: ManagedAddress, new_permissions: Permissions) {
        self.permissions(address).update(|permissions| {
            permissions.insert(new_permissions);
        });
    }

    fn remove_permissions(&self, address: ManagedAddress, permissions_to_remove: Permissions) {
        self.permissions(address).update(|permissions| {
            permissions.remove(permissions_to_remove);
        });
    }

    fn add_permissions_for_all(
        &self,
        addresses: MultiValueEncoded<ManagedAddress>,
        permissions: Permissions,
    ) {
        for address in addresses {
            self.add_permissions(address, permissions);
        }
    }

    fn require_caller_any_of(&self, permissions: Permissions) {
        let caller = self.blockchain().get_caller();
        let caller_permissions = self.permissions(caller).get();
        require!(
            caller_permissions.intersects(permissions),
            ERROR_PERMISSION_DENIED
        );
    }

    fn require_caller_has_owner_permissions(&self) {
        self.require_caller_any_of(Permissions::OWNER);
    }

    fn require_caller_has_owner_or_admin_permissions(&self) {
        self.require_caller_any_of(Permissions::OWNER | Permissions::ADMIN);
    }

    fn require_caller_has_admin_permissions(&self) {
        self.require_caller_any_of(Permissions::ADMIN);
    }

    fn require_caller_has_pause_permissions(&self) {
        self.require_caller_any_of(Permissions::PAUSE);
    }

    #[view(getPermissions)]
    #[storage_mapper("permissions")]
    fn permissions(&self, address: ManagedAddress) -> SingleValueMapper<Permissions>;
}
