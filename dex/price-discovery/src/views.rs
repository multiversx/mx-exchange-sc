multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ViewsModule: crate::common_storage::CommonStorageModule {
    #[view(getCurrentPrice)]
    fn get_current_price(&self) -> BigUint {
        let launched_token_balance = self.launched_token_balance().get();
        let accepted_token_balance = self.accepted_token_balance().get();

        require!(launched_token_balance > 0, "No launched tokens available");

        let price_precision = self.price_precision().get();
        accepted_token_balance * price_precision / launched_token_balance
    }
}
