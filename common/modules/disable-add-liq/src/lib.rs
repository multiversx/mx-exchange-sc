#![no_std]

multiversx_sc::imports!();

pub type AddLiqStatus = bool;
pub const ADD_LIQ_ENABLED: AddLiqStatus = true;
pub const ADD_LIQ_DISABLED: AddLiqStatus = false;

#[multiversx_sc::module]
pub trait DisableAddLiqModule {
    #[only_owner]
    #[endpoint(enableAddLiq)]
    fn enable_add_liq(&self) {
        self.add_liq_enabled().set(ADD_LIQ_ENABLED);
    }

    #[only_owner]
    #[endpoint(disableAddLiq)]
    fn disable_add_liq(&self) {
        self.add_liq_enabled().set(ADD_LIQ_DISABLED);
    }

    fn require_add_liq_enabled(&self) {
        require!(
            self.add_liq_enabled().get() == ADD_LIQ_ENABLED,
            "Add Liquidity is disabled"
        );
    }

    #[view(isAddLiqEnabled)]
    #[storage_mapper("addLiqEnabled")]
    fn add_liq_enabled(&self) -> SingleValueMapper<AddLiqStatus>;
}
