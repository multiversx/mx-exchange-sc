#![no_std]

multiversx_sc::imports!();

use weekly_rewards_splitting::update_claim_progress_energy::ProxyTrait as _;

#[multiversx_sc::contract]
pub trait EnergyUpdate {
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[endpoint(updateFarmsEnergyForUser)]
    fn update_farms_energy_for_user(
        &self,
        user: ManagedAddress,
        farm_addresses: MultiValueEncoded<ManagedAddress>,
    ) {
        for farm_addr in farm_addresses {
            self.farm_proxy(farm_addr)
                .update_energy_for_user(user.clone())
                .sync_call();
        }
    }

    #[proxy]
    fn farm_proxy(&self, user: ManagedAddress) -> farm::Proxy<Self::Api>;
}
