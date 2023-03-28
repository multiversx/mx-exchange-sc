use crate::tokens_per_user::UnstakePair;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait EventsModule {
    fn emit_unlocked_tokens_event(
        &self,
        caller: &ManagedAddress,
        unlocked_tokens: ManagedVec<UnstakePair<Self::Api>>,
    ) {
        self.unlocked_tokens_event(
            caller,
            self.blockchain().get_block_nonce(),
            self.blockchain().get_block_epoch(),
            self.blockchain().get_block_timestamp(),
            unlocked_tokens,
        );
    }

    #[event("userUnlockedTokens")]
    fn unlocked_tokens_event(
        &self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] block: u64,
        #[indexed] epoch: u64,
        #[indexed] timestamp: u64,
        data: ManagedVec<UnstakePair<Self::Api>>,
    );
}
