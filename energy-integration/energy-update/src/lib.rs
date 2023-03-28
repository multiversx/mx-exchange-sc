#![no_std]

multiversx_sc::imports!();

use weekly_rewards_splitting::update_claim_progress_energy::ProxyTrait as _;

#[multiversx_sc::contract]
pub trait EnergyUpdate {
    #[init]
    fn init(&self) {}

    #[endpoint(updateFarmsEnergyForUser)]
    fn update_farms_energy_for_user(
        &self,
        user: ManagedAddress,
        farm_addresses: MultiValueEncoded<ManagedAddress>,
    ) {
        for farm_addr in farm_addresses {
            let _: IgnoreValue = self
                .farm_proxy(farm_addr)
                .update_energy_for_user(user.clone())
                .execute_on_dest_context();
        }
    }

    #[proxy]
    fn farm_proxy(&self, user: ManagedAddress) -> farm::Proxy<Self::Api>;
}
