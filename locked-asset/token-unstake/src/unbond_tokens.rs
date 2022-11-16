elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait UnbondTokensModule: crate::tokens_per_user::TokensPerUserModule {
    #[endpoint(claimUnlockedTokens)]
    fn claim_unlocked_tokens(&self) -> MultiValueEncoded<EsdtTokenPayment> {
        let caller = self.blockchain().get_caller();
        let current_epoch = self.blockchain().get_block_epoch();
        let mut output_payments = ManagedVec::new();
        let mut penalty_tokens = ManagedVec::new();
        self.unlocked_tokens_for_user(&caller)
            .update(|user_entries| {
                while !user_entries.is_empty() {
                    let entry = user_entries.get(0);
                    if current_epoch < entry.unlock_epoch {
                        break;
                    }

                    let locked_tokens = entry.locked_tokens;
                    let unlocked_tokens = entry.unlocked_tokens;

                    // we only burn the tokens that are not unlocked
                    // the rest are sent back as penalty
                    let locked_tokens_burn_amount = unlocked_tokens.amount.clone();
                    self.send().esdt_local_burn(
                        &locked_tokens.token_identifier,
                        locked_tokens.token_nonce,
                        &locked_tokens_burn_amount,
                    );

                    let penalty_amount = &locked_tokens.amount - &unlocked_tokens.amount;
                    if penalty_amount > 0 {
                        let penalty = EsdtTokenPayment::new(
                            locked_tokens.token_identifier,
                            locked_tokens.token_nonce,
                            penalty_amount,
                        );
                        penalty_tokens.push(penalty);
                    }

                    output_payments.push(unlocked_tokens);
                    user_entries.remove(0);
                }
            });

        if !output_payments.is_empty() {
            self.send().direct_multi(&caller, &output_payments);
        }

        if !penalty_tokens.is_empty() {
            let sc_address = self.energy_factory_address().get();
            let _: IgnoreValue = self
                .energy_factory_proxy(sc_address)
                .finalize_unstake()
                .with_multi_token_transfer(penalty_tokens)
                .execute_on_dest_context();
        }

        output_payments.into()
    }

    #[view(getUnbondEpochs)]
    #[storage_mapper("unbondEpochs")]
    fn unbond_epochs(&self) -> SingleValueMapper<u64>;
}
