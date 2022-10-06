elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use fixed_supply_token::FixedSupplyToken;
use mergeable::Mergeable;

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct WrappedFarmTokenAttributes<M: ManagedTypeApi> {
    pub farm_token: EsdtTokenPayment<M>,
    pub proxy_farming_token: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> FixedSupplyToken<M> for WrappedFarmTokenAttributes<M> {
    fn get_total_supply(&self) -> &BigUint<M> {
        &self.farm_token.amount
    }

    fn into_part(self, payment_amount: &BigUint<M>) -> Self {
        todo!()
    }
}

impl<M: ManagedTypeApi> Mergeable<M> for WrappedFarmTokenAttributes<M> {
    fn can_merge_with(&self, other: &Self) -> bool {
        todo!()
    }

    fn merge_with(&mut self, other: Self) {
        todo!()
    }
}
