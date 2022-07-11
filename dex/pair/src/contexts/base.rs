elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::State;

pub struct StorageCache<'a, C>
where
    C: crate::config::ConfigModule,
{
    sc_ref: &'a C,
    pub contract_state: State,
    pub lp_token_id: TokenIdentifier<C::Api>,
    pub first_token_id: TokenIdentifier<C::Api>,
    pub second_token_id: TokenIdentifier<C::Api>,
    pub first_token_reserve: BigUint<C::Api>,
    pub second_token_reserve: BigUint<C::Api>,
    pub lp_token_supply: BigUint<C::Api>,
}

impl<'a, C> StorageCache<'a, C>
where
    C: crate::config::ConfigModule,
{
    pub fn new(sc_ref: &'a C) -> Self {
        let first_token_id = sc_ref.first_token_id().get();
        let second_token_id = sc_ref.second_token_id().get();
        let first_token_reserve = sc_ref.pair_reserve(&first_token_id).get();
        let second_token_reserve = sc_ref.pair_reserve(&second_token_id).get();

        StorageCache {
            contract_state: sc_ref.state().get(),
            lp_token_id: sc_ref.lp_token_identifier().get(),
            first_token_id,
            second_token_id,
            first_token_reserve,
            second_token_reserve,
            lp_token_supply: sc_ref.lp_token_supply().get(),
            sc_ref,
        }
    }
}

impl<'a, C> Drop for StorageCache<'a, C>
where
    C: crate::config::ConfigModule,
{
    fn drop(&mut self) {
        // commit changes to storage for the mutable fields
        self.sc_ref
            .pair_reserve(&self.first_token_id)
            .set(&self.first_token_reserve);

        self.sc_ref
            .pair_reserve(&self.second_token_id)
            .set(&self.second_token_reserve);

        self.sc_ref.lp_token_supply().set(&self.lp_token_supply);
    }
}
