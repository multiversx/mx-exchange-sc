#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub const DEFAULT_UNBOND_EPOCHS: u64 = 10;

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct UnstakePair<M: ManagedTypeApi> {
    pub unlock_epoch: u64,
    pub token_payment: EsdtTokenPayment<M>,
}

#[elrond_wasm::contract]
pub trait TokenUnstakeModule: token_send::TokenSendModule {
    #[init]
    fn init(&self) {
        self.unbond_epochs().set_if_empty(DEFAULT_UNBOND_EPOCHS);
    }

    #[only_owner]
    #[endpoint(setUnbondEpochs)]
    fn set_unbond_epochs(&self, unbond_epochs: u64) {
        self.unbond_epochs().set(unbond_epochs);
    }

    #[payable("*")]
    #[endpoint(depositUserTokens)]
    fn deposit_user_tokens(&self, caller: &ManagedAddress) {
        let token_payment = self.call_value().single_esdt();
        require!(
            token_payment.token_nonce == 0,
            "Can only unstake fungible tokens"
        );
        require!(
            token_payment.amount > 0,
            "Payment amount must be greater than 0"
        );
        let current_epoch = self.blockchain().get_block_epoch();
        let unbond_epochs = self.unbond_epochs().get();
        let unstake_pair = UnstakePair {
            unlock_epoch: current_epoch + unbond_epochs,
            token_payment,
        };
        self.unlocked_tokens_for_user(&caller)
            .update(|unstake_pairs| {
                unstake_pairs.push(unstake_pair);
            });
    }

    #[endpoint(claimUnlockedTokens)]
    fn claim_unlocked_tokens(&self) {
        let caller = self.blockchain().get_caller();
        let current_epoch = self.blockchain().get_block_epoch();
        let unlocked_tokens_for_user_mapper = self.unlocked_tokens_for_user(&caller);
        let unlocked_tokens_for_user = unlocked_tokens_for_user_mapper.get();
        let mut remaining_tokens_for_user = ManagedVec::new();
        let mut payments = ManagedVec::new();
        for unstake_payment in &unlocked_tokens_for_user {
            if current_epoch >= unstake_payment.unlock_epoch {
                payments.push(unstake_payment.token_payment);
            } else {
                remaining_tokens_for_user.push(unstake_payment);
            }
        }
        if remaining_tokens_for_user.is_empty() {
            unlocked_tokens_for_user_mapper.clear();
        } else {
            unlocked_tokens_for_user_mapper.set(remaining_tokens_for_user);
        };
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
