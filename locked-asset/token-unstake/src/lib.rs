#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct UnstakePair<M: ManagedTypeApi> {
    pub unlock_epoch: u64,
    pub token_payment: EsdtTokenPayment<M>,
}

#[elrond_wasm::contract]
pub trait TokenUnstakeModule: token_send::TokenSendModule {
    #[init]
    fn init(&self, unbond_epochs: u64) {
        self.unbond_epochs().set_if_empty(unbond_epochs);
    }

    #[only_owner]
    #[endpoint(setUnbondEpochs)]
    fn set_unbond_epochs(&self, unbond_epochs: u64) {
        self.unbond_epochs().set(unbond_epochs);
    }

    #[only_owner]
    #[endpoint(addTokenToWhitelist)]
    fn add_unstake_tokens_to_whitelist(&self, tokens: MultiValueEncoded<TokenIdentifier>) {
        let token_mapper = self.unstake_tokens();
        for token in tokens {
            require!(!token_mapper.contains(&token), "Token already whitelisted");
            token_mapper.add(&token);
        }
    }

    #[only_owner]
    #[endpoint(removeSCAddressFromWhitelist)]
    fn remove_unstake_tokens_from_whitelist(&self, token: TokenIdentifier) {
        let token_mapper = self.unstake_tokens();
        require!(token_mapper.contains(&token), "Token is not whitelisted");
        token_mapper.remove(&token);
    }

    #[inline]
    fn require_token_whitelisted(&self, token: &TokenIdentifier) {
        self.unstake_tokens().require_whitelisted(token);
    }

    #[payable("*")]
    #[endpoint(depositUserTokens)]
    fn deposit_user_tokens(&self, caller: &ManagedAddress) {
        let token_payment = self.call_value().single_esdt();
        self.require_token_whitelisted(&token_payment.token_identifier);
        require!(
            token_payment.token_nonce == 0,
            "Can only unstake fungible tokens"
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
        unlocked_tokens_for_user_mapper.set(remaining_tokens_for_user);
        self.send_multiple_tokens_if_not_zero(&caller, &payments);
    }

    #[view(getUnbondEpochs)]
    #[storage_mapper("unbondEpochs")]
    fn unbond_epochs(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("unstakeTokens")]
    fn unstake_tokens(&self) -> WhitelistMapper<Self::Api, TokenIdentifier>;

    #[view(getUnlockedTokensForUser)]
    #[storage_mapper("unlockedTokensForUser")]
    fn unlocked_tokens_for_user(
        &self,
        address: &ManagedAddress,
    ) -> SingleValueMapper<ManagedVec<UnstakePair<Self::Api>>>;
}
