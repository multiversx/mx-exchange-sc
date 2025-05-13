#![no_std]

use permissions_module::Permissions;

multiversx_sc::imports!();

pub mod events;
pub mod views;
pub mod whitelist;

#[multiversx_sc::contract]
pub trait WhitelistHub:
    permissions_module::PermissionsModule
    + crate::whitelist::WhitelistModule
    + crate::views::ViewsModule
    + crate::events::EventsModule
{
    #[init]
    fn init(&self) {
        let caller = self.blockchain().get_caller();
        self.add_permissions(caller, Permissions::OWNER | Permissions::ADMIN);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
