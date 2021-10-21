elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub type Nonce = u64;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, TypeAbi)]
pub struct ReserveNode<M: ManagedTypeApi> {
    pub amount: BigUint<M>,
    pub queue_id: u64,
    pub next: Nonce,
    pub prev: Nonce,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, TypeAbi)]
pub struct NonceAmountPair<M: ManagedTypeApi> {
    pub amount: BigUint<M>,
    pub nonce: Nonce,
}

impl<M: ManagedTypeApi> ReserveNode<M> {
    pub fn from(amount: BigUint<M>, queue_id: u64) -> Self {
        ReserveNode {
            amount,
            queue_id,
            next: 0,
            prev: 0,
        }
    }
}

impl<M: ManagedTypeApi> NonceAmountPair<M> {
    pub fn from(nonce: Nonce, amount: BigUint<M>) -> Self {
        NonceAmountPair { amount, nonce }
    }
}
