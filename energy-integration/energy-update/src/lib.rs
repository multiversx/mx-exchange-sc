#![no_std]

elrond_wasm::imports!();

mod farm_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait Farm {
        #[endpoint(updateEnergyForUser)]
        fn update_energy_for_user(&self, user: ManagedAddress);
    }
}

#[elrond_wasm::contract]
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
