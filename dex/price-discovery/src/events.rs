elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait EventsModule: crate::common_storage::CommonStorageModule {
    fn emit_deposit_extra_rewards_event(&self, token_id: &TokenIdentifier, amount: &BigUint) {
        let caller = self.blockchain().get_caller();
        let current_block = self.blockchain().get_block_nonce();

        self.deposit_extra_rewards_event(current_block, &caller, token_id, amount);
    }

    fn emit_deposit_event(
        &self,
        token_id_in: &TokenIdentifier,
        amount_in: &BigUint,
        redeem_token_nonce_out: u64,
        redeem_token_amount_out: &BigUint,
        current_price: &BigUint,
    ) {
        let current_block = self.blockchain().get_block_nonce();
        let caller = self.blockchain().get_caller();
        let launched_token_balance = self.launched_token_balance().get();
        let accepted_token_balance = self.accepted_token_balance().get();

        self.deposit_event(
            current_block,
            &caller,
            token_id_in,
            amount_in,
            redeem_token_nonce_out,
            redeem_token_amount_out,
            &launched_token_balance,
            &accepted_token_balance,
            current_price,
        );
    }

    fn emit_withdraw_event(
        &self,
        redeem_token_nonce_in: u64,
        redeem_token_amount_in: &BigUint,
        token_id_out: &TokenIdentifier,
        token_amount_out: &BigUint,
        current_price: &BigUint,
    ) {
        let current_block = self.blockchain().get_block_nonce();
        let caller = self.blockchain().get_caller();
        let launched_token_balance = self.launched_token_balance().get();
        let accepted_token_balance = self.accepted_token_balance().get();

        self.withdraw_event(
            current_block,
            &caller,
            redeem_token_nonce_in,
            redeem_token_amount_in,
            token_id_out,
            token_amount_out,
            &launched_token_balance,
            &accepted_token_balance,
            current_price,
        );
    }

    #[event("depositExtraRewardsEvent")]
    fn deposit_extra_rewards_event(
        &self,
        #[indexed] block: u64,
        #[indexed] caller: &ManagedAddress,
        #[indexed] token_id: &TokenIdentifier,
        amount: &BigUint,
    );

    #[event("depositEvent")]
    fn deposit_event(
        &self,
        #[indexed] block: u64,
        #[indexed] caller: &ManagedAddress,
        #[indexed] token_id_in: &TokenIdentifier,
        #[indexed] amount_in: &BigUint,
        #[indexed] redeem_token_nonce_out: u64,
        #[indexed] redeem_token_amount_out: &BigUint,
        #[indexed] launched_token_balance: &BigUint,
        #[indexed] accepted_token_balance: &BigUint,
        current_price: &BigUint,
    );

    #[event("withdrawEvent")]
    fn withdraw_event(
        &self,
        #[indexed] block: u64,
        #[indexed] caller: &ManagedAddress,
        #[indexed] redeem_token_nonce_in: u64,
        #[indexed] redeem_token_amount_in: &BigUint,
        #[indexed] token_id_out: &TokenIdentifier,
        #[indexed] token_amount_out: &BigUint,
        #[indexed] launched_token_balance: &BigUint,
        #[indexed] accepted_token_balance: &BigUint,
        current_price: &BigUint,
    );

    #[event("liquidityPoolCreatedEvent")]
    fn liquidity_pool_created_event(
        &self,
        #[indexed] creation_epoch: u64,
        #[indexed] unbond_period_end_epoch: u64,
        #[indexed] launched_token_final_amount: &BigUint,
        #[indexed] accepted_token_final_amount: &BigUint,
        #[indexed] extra_rewards_final_amount: &BigUint,
        #[indexed] lp_token_id: &TokenIdentifier,
        total_lp_tokens_received: &BigUint,
    );

    #[event("redeemEvent")]
    fn redeem_event(
        &self,
        #[indexed] block: u64,
        #[indexed] caller: &ManagedAddress,
        #[indexed] redeem_token_nonce_in: u64,
        #[indexed] redeem_token_amount_in: &BigUint,
        #[indexed] extra_rewards_out: &BigUint,
        #[indexed] lp_tokens_out: &BigUint,
        lp_tokens_remaining: &BigUint,
    );
}
