#![no_std]

multiversx_sc::imports!();

pub const ADD_LIQ_ENABLED: bool = false;
pub const ADD_LIQ_DISABLED: bool = true;

#[multiversx_sc::module]
pub trait DisableAddLiqModule {
    #[only_owner]
    #[endpoint(enableAddLiq)]
    fn enable_add_liq(&self) {
        self.add_liq_disabled().set(ADD_LIQ_ENABLED);
    }

    #[only_owner]
    #[endpoint(disableAddLiq)]
    fn disable_add_liq(&self) {
        self.add_liq_disabled().set(ADD_LIQ_DISABLED);
    }

    fn require_add_liq_enabled(&self) {
        require!(
            self.add_liq_disabled().get() == ADD_LIQ_ENABLED,
            "Add Liquidity is disabled"
        );
    }

    #[view(isAddLiqDisabled)]
    #[storage_mapper("addLiqDisabled")]
    fn add_liq_disabled(&self) -> SingleValueMapper<bool>;
}
