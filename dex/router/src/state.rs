use pair::read_pair_storage;

use pausable::ProxyTrait as _;

multiversx_sc::imports!();

pub type State = bool;
pub const ACTIVE: State = true;
pub const INACTIVE: State = false;

#[multiversx_sc::module]
pub trait StateModule:
    crate::config::ConfigModule + read_pair_storage::ReadPairStorageModule
{
    #[only_owner]
    #[endpoint]
    fn pause(&self, address: ManagedAddress) {
        if address == self.blockchain().get_sc_address() {
            self.state().set(INACTIVE);
        } else {
            self.check_is_pair_sc(&address);

            let _: IgnoreValue = self
                .pair_contract_proxy_state(address)
                .pause()
                .execute_on_dest_context();
        }
    }

    #[only_owner]
    #[endpoint]
    fn resume(&self, address: ManagedAddress) {
        if address == self.blockchain().get_sc_address() {
            self.state().set(ACTIVE);
        } else {
            self.check_is_pair_sc(&address);
            let _: IgnoreValue = self
                .pair_contract_proxy_state(address)
                .resume()
                .execute_on_dest_context();
        }
    }

    fn require_active(&self) {
        require!(self.state().get() == ACTIVE, "Not active");
    }

    #[proxy]
    fn pair_contract_proxy_state(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<State>;
}
