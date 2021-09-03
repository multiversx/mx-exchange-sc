elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub type Nonce = u64;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, TypeAbi)]
pub struct ReserveNode<BigUint: BigUintApi> {
    pub amount: BigUint,
    pub queue_id: u64,
    pub next: Nonce,
    pub prev: Nonce,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, TypeAbi)]
pub struct NonceAmountPair<BigUint: BigUintApi> {
    pub amount: BigUint,
    pub nonce: Nonce,
}

impl<BigUint: BigUintApi> ReserveNode<BigUint> {
    pub fn new() -> Self {
        ReserveNode::default()
    }

    pub fn from(amount: BigUint, queue_id: u64) -> Self {
        ReserveNode {
            amount,
            queue_id,
            next: 0,
            prev: 0,
        }
    }
}

impl<BigUint: BigUintApi> Default for ReserveNode<BigUint> {
    fn default() -> Self {
        ReserveNode {
            amount: BigUint::zero(),
            queue_id: 0,
            next: 0,
            prev: 0,
        }
    }
}

impl<BigUint: BigUintApi> NonceAmountPair<BigUint> {
    pub fn new() -> Self {
        NonceAmountPair::default()
    }

    pub fn from(nonce: Nonce, amount: BigUint) -> Self {
        NonceAmountPair { amount, nonce }
    }
}

impl<BigUint: BigUintApi> Default for NonceAmountPair<BigUint> {
    fn default() -> Self {
        NonceAmountPair {
            amount: BigUint::zero(),
            nonce: 0,
        }
    }
}
