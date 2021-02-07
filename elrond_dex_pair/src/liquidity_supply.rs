imports!();
derive_imports!();


#[elrond_wasm_derive::module(LiquiditySupplyModuleImpl)]
pub trait LiquiditySupplyModule {

    #[storage_get("total_supply")]
    fn get_total_supply(&self) -> BigUint;

    #[storage_set("total_supply")]
    fn set_total_supply(&self, total_supply: &BigUint);

    #[storage_get("balance_of")]
    fn get_balance_of(&self, address: &Address) -> BigUint;

    #[storage_set("balance_of")]
    fn set_balance_of(&self, address: &Address, balance: &BigUint);

    fn _mint(&self, to: &Address,
        value: &BigUint) {
        let mut total_supply = self.get_total_supply();
        let mut balance_of = self.get_balance_of(to);

        total_supply += value;
        balance_of += value;

        self.set_total_supply(&total_supply);
        self.set_balance_of(to, &balance_of);
    }

    fn _burn(&self, from: &Address, value: &BigUint) {
        let mut total_supply = self.get_total_supply();
        let mut balance_of = self.get_balance_of(from);

        total_supply-= value;
        balance_of -= value;

        self.set_total_supply(&total_supply);
        self.set_balance_of(from, &balance_of);
    }

}