elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi, Eq)]
pub struct TokenPair<M: ManagedTypeApi> {
    pub first_token: TokenIdentifier<M>,
    pub second_token: TokenIdentifier<M>,
}

impl<M: ManagedTypeApi> TokenPair<M> {
    pub fn equals(&self, other: &TokenPair<M>) -> bool {
        self.first_token == other.first_token && self.second_token == other.second_token
    }
}

#[derive(ManagedVecItem, Clone, Debug)]
pub struct EpochAmountPair<M: ManagedTypeApi> {
    pub epoch: u64,
    pub amount: BigUint<M>,
}
