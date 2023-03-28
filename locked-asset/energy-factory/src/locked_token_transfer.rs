multiversx_sc::imports!();

use crate::energy::Energy;

#[multiversx_sc::module]
pub trait LockedTokenTransferModule:
    utils::UtilsModule + crate::energy::EnergyModule + crate::events::EventsModule
{
    #[only_owner]
    #[endpoint(addToTokenTransferWhitelist)]
    fn add_to_token_transfer_whitelist(&self, sc_addresses: MultiValueEncoded<ManagedAddress>) {
        let mapper = self.token_transfer_whitelist();
        for sc_addr in sc_addresses {
            self.require_sc_address(&sc_addr);
            mapper.add(&sc_addr);
        }
    }

    #[only_owner]
    #[endpoint(removeFromTokenTransferWhitelist)]
    fn remove_from_token_transfer_whitelist(
        &self,
        sc_addresses: MultiValueEncoded<ManagedAddress>,
    ) {
        let mapper = self.token_transfer_whitelist();
        for sc_addr in sc_addresses {
            mapper.remove(&sc_addr);
        }
    }

    #[endpoint(setUserEnergyAfterLockedTokenTransfer)]
    fn set_user_energy_after_locked_token_transfer(
        &self,
        user: ManagedAddress,
        energy: Energy<Self::Api>,
    ) {
        let caller = self.blockchain().get_caller();
        self.token_transfer_whitelist().require_whitelisted(&caller);

        self.set_energy_entry(&user, energy);
    }

    #[storage_mapper("tokenTransferWhitelist")]
    fn token_transfer_whitelist(&self) -> WhitelistMapper<ManagedAddress>;
}
