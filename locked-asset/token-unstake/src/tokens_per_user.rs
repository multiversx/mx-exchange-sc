elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct UnstakePair<M: ManagedTypeApi> {
    pub unlock_epoch: u64,
    pub locked_tokens: EsdtTokenPayment<M>,
    pub unlocked_tokens: EsdtTokenPayment<M>,
}

#[elrond_wasm::module]
pub trait TokensPerUserModule {
    #[view(getUnlockedTokensForUser)]
    #[storage_mapper("unlockedTokensForUser")]
    fn unlocked_tokens_for_user(
        &self,
        address: &ManagedAddress,
    ) -> SingleValueMapper<ManagedVec<UnstakePair<Self::Api>>>;
}