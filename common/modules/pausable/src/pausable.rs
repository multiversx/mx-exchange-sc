#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Copy, Clone, Debug)]
pub enum State {
    Inactive,
    Active,
    PartialActive,
}

#[elrond_wasm::module]
pub trait PausableModule {
    #[endpoint(addToPauseWhitelist)]
    fn add_to_pause_whitelist(&self, addr_list: MultiValueEncoded<ManagedAddress>) {
        self.require_caller_in_pause_whitelist();

        let whitelist = self.pause_whitelist();
        for addr in addr_list {
            whitelist.add(&addr);
        }
    }

    #[endpoint(removeFromPauseWhitelist)]
    fn remove_from_pause_whitelist(&self, addr_list: MultiValueEncoded<ManagedAddress>) {
        self.require_caller_in_pause_whitelist();

        let whitelist = self.pause_whitelist();
        for addr in addr_list {
            whitelist.remove(&addr);
        }
    }

    #[endpoint]
    fn pause(&self) {
        self.require_caller_in_pause_whitelist();
        self.state().set(State::Inactive);
    }

    #[endpoint]
    fn resume(&self) {
        self.require_caller_in_pause_whitelist();
        self.state().set(State::Active);
    }

    fn require_caller_in_pause_whitelist(&self) {
        let caller = self.blockchain().get_caller();
        self.pause_whitelist().require_whitelisted(&caller);
    }

    #[storage_mapper("pauseWhitelist")]
    fn pause_whitelist(&self) -> WhitelistMapper<Self::Api, ManagedAddress>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<State>;
}
