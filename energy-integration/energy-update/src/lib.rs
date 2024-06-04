#![no_std]

multiversx_sc::imports!();
mod farm_proxy;

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
            self.tx()
                .to(&farm_addr)
                .typed(farm_proxy::FarmProxy)
                .update_energy_for_user(user.clone())
                .sync_call();
        }
    }
}
