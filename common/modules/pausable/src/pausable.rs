#![no_std]

use permissions_module::Permissions;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Copy, Clone, Debug)]
pub enum State {
    Inactive,
    Active,
    PartialActive,
}

#[elrond_wasm::module]
pub trait PausableModule: permissions_module::PermissionsModule {
    #[endpoint(addToPauseWhitelist)]
    fn add_to_pause_whitelist(&self, address_list: MultiValueEncoded<ManagedAddress>) {
        self.require_caller_has_owner_permissions();

        for address in address_list {
            self.add_permissions(address, Permissions::PAUSE);
        }
    }

    #[endpoint(removeFromPauseWhitelist)]
    fn remove_from_pause_whitelist(&self, address_list: MultiValueEncoded<ManagedAddress>) {
        self.require_caller_has_owner_permissions();

        for address in address_list {
            self.remove_permissions(address, Permissions::PAUSE);
        }
    }

    #[endpoint]
    fn pause(&self) {
        self.require_caller_has_pause_permissions();
        self.state().set(State::Inactive);
    }

    #[endpoint]
    fn resume(&self) {
        self.require_caller_has_pause_permissions();
        self.state().set(State::Active);
    }

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<State>;
}
