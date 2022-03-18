elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait EventsModule {
    #[event("depositExtraRewardsEvent")]
    fn deposit_extra_rewards_event(
        &self,
        #[indexed] block: u64,
        #[indexed] caller: &ManagedAddress,
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
