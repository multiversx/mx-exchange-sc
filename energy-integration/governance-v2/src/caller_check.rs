multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait CallerCheckModule {
    fn require_caller_not_self(&self) {
        let caller = self.blockchain().get_caller();
        let sc_address = self.blockchain().get_sc_address();

        require!(
            caller != sc_address,
            "Cannot call this endpoint through proposed action"
        );
    }
}
