elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait CallerCheckModule {
    fn require_caller_not_self(&self) {
        let caller = self.blockchain().get_caller();
        let sc_address = self.blockchain().get_sc_address();

        require!(
            caller != sc_address,
            "Cannot call this endpoint through proposed action"
        );
    }

    fn require_caller_self(&self) {
        let caller = self.blockchain().get_caller();
        let sc_address = self.blockchain().get_sc_address();

        require!(
            caller == sc_address,
            "Only the SC itself may call this function"
        );
    }
}
