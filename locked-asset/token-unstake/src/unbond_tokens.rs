use common_structs::PaymentsVec;

use crate::{events, tokens_per_user::UnstakePair};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait UnbondTokensModule:
    crate::tokens_per_user::TokensPerUserModule
    + crate::fees_handler::FeesHandlerModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
    + events::EventsModule
{
    #[endpoint(claimUnlockedTokens)]
    fn claim_unlocked_tokens(&self) -> MultiValueEncoded<EsdtTokenPayment> {
        let caller = self.blockchain().get_caller();
        let current_epoch = self.blockchain().get_block_epoch();
        let mut output_payments = ManagedVec::new();
        let mut penalty_tokens = ManagedVec::<Self::Api, _>::new();
        let new_unlocked_tokens = self
            .unlocked_tokens_for_user(&caller)
            .update(|user_entries| {
                while !user_entries.is_empty() {
                    let entry = user_entries.get(0);
                    if current_epoch < entry.unlock_epoch {
                        break;
                    }

                    self.handle_single_unbond_entry(&entry, &mut penalty_tokens);

                    let unlocked_tokens = entry.unlocked_tokens;
                    output_payments.push(unlocked_tokens);
                    user_entries.remove(0);
                }

                (*user_entries).clone()
            });

        require!(!output_payments.is_empty(), "Nothing to unbond");

        for token in &penalty_tokens {
            self.burn_penalty(token);
        }

        self.send().direct_multi(&caller, &output_payments);
        self.emit_unlocked_tokens_event(&caller, new_unlocked_tokens);

        output_payments.into()
    }

    fn handle_single_unbond_entry(
        &self,
        entry: &UnstakePair<Self::Api>,
        penalty_tokens: &mut PaymentsVec<Self::Api>,
    ) {
        let locked_tokens = &entry.locked_tokens;
        let unlocked_tokens = &entry.unlocked_tokens;

        // we only burn the tokens that are not unlocked
        // the rest are sent back as penalty
        let locked_tokens_burn_amount = &unlocked_tokens.amount;
        self.send().esdt_local_burn(
            &locked_tokens.token_identifier,
            locked_tokens.token_nonce,
            locked_tokens_burn_amount,
        );

        let penalty_amount = &locked_tokens.amount - &unlocked_tokens.amount;
        if penalty_amount == 0 {
            return;
        }

        let penalty = EsdtTokenPayment::new(
            locked_tokens.token_identifier.clone(),
            locked_tokens.token_nonce,
            penalty_amount,
        );
        penalty_tokens.push(penalty);
    }
}
