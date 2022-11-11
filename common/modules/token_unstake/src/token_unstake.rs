#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct UnstakePair<M: ManagedTypeApi> {
    pub unlock_epoch: u64,
    pub token_payment: EsdtTokenPayment<M>,
}

#[elrond_wasm::module]
pub trait TokenUnstakeModule: token_send::TokenSendModule {
    #[only_owner]
    #[endpoint(setUnbondEpochs)]
    fn set_unbond_epochs(&self, unbond_epochs: u64) {
        self.unbond_epochs().set(unbond_epochs);
    }

    #[endpoint(claimUnlockedTokens)]
    fn claim_unlocked_tokens(&self) {
        let caller = self.blockchain().get_caller();
        let current_epoch = self.blockchain().get_block_epoch();
        let unlocked_tokens_for_user_mapper = self.unlocked_tokens_for_user(&caller);
        let unlocked_tokens_for_user = unlocked_tokens_for_user_mapper.get();
        let mut remaining_tokens_for_user = unlocked_tokens_for_user.clone();
        let mut payments = ManagedVec::new();
        for unstake_payment in &unlocked_tokens_for_user {
            if unstake_payment.unlock_epoch > current_epoch {
                break;
            }
            payments.push(unstake_payment.token_payment);
            remaining_tokens_for_user.remove(0);
        }

        unlocked_tokens_for_user_mapper.set(remaining_tokens_for_user);
        self.send_multiple_tokens_if_not_zero(&caller, &payments);
    }

    #[view(getUnbondEpochs)]
    #[storage_mapper("unbondEpochs")]
    fn unbond_epochs(&self) -> SingleValueMapper<u64>;

    #[view(getUnlockedTokensForUser)]
    #[storage_mapper("unlockedTokensForUser")]
    fn unlocked_tokens_for_user(
        &self,
        address: &ManagedAddress,
    ) -> SingleValueMapper<ManagedVec<UnstakePair<Self::Api>>>;
}
