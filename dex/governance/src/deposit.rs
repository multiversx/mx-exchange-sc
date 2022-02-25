elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq)]
pub enum DepositIntent {
    Vote = 1,
    Downvote = 2,
    Action = 3,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi)]
pub struct UserDeposit<M: ManagedTypeApi> {
    pub payment: EsdtTokenPayment<M>,
    pub mex_equivalent: BigUint<M>,
    pub intent: DepositIntent,
}
