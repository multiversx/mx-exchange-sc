elrond_wasm::imports!();

use common_structs::{Epoch, Nonce, NonceAmountPair, PaymentsVec};

use crate::lock_options::MAX_PENALTY_PERCENTAGE;

const EPOCHS_PER_WEEK: Epoch = 7;

pub mod fees_collector_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait FeesCollectorProxy {
        #[payable("*")]
        #[endpoint(depositSwapFees)]
        fn deposit_swap_fees(&self);
    }
}

#[elrond_wasm::module]
pub trait FeesModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::token_merging::TokenMergingModule
    + elrond_wasm_modules::pause::PauseModule
    + crate::penalty::LocalPenaltyModule
    + crate::energy::EnergyModule
    + crate::events::EventsModule
    + crate::lock_options::LockOptionsModule
    + utils::UtilsModule
{
    fn burn_penalty(&self, token_id: TokenIdentifier, token_nonce: Nonce, fees_amount: &BigUint) {
        let fees_burn_percentage = self.fees_burn_percentage().get();
        let burn_amount = fees_amount * fees_burn_percentage as u64 / MAX_PENALTY_PERCENTAGE as u64;
        let remaining_amount = fees_amount - &burn_amount;

        if burn_amount > 0 {
            self.send()
                .esdt_local_burn(&token_id, token_nonce, &burn_amount);
        }
        if remaining_amount > 0 {
            if self.fees_from_penalty_unlocking().is_empty() {
                // First fee deposit of the week
                self.fees_from_penalty_unlocking()
                    .set(NonceAmountPair::new(token_nonce, remaining_amount));
            } else {
                self.merge_fees_from_penalty(token_nonce, remaining_amount);
            }
        }

        // Only once per week
        self.send_fees_to_collector();
    }

    /// Merges new fees with existing fees and saves in storage
    fn merge_fees_from_penalty(&self, token_nonce: Nonce, new_fee_amount: BigUint) {
        let locked_token_mapper = self.locked_token();
        let existing_nonce_amount_pair = self.fees_from_penalty_unlocking().get();
        let mut payments = PaymentsVec::new();
        payments.push(EsdtTokenPayment::new(
            locked_token_mapper.get_token_id(),
            token_nonce,
            new_fee_amount,
        ));
        payments.push(EsdtTokenPayment::new(
            locked_token_mapper.get_token_id(),
            existing_nonce_amount_pair.nonce,
            existing_nonce_amount_pair.amount,
        ));

        let new_locked_amount_attributes = self.merge_tokens(payments, OptionalValue::None);

        let sft_nonce = self.get_or_create_nonce_for_attributes(
            &locked_token_mapper,
            &new_locked_amount_attributes
                .attributes
                .original_token_id
                .clone()
                .into_name(),
            &new_locked_amount_attributes.attributes,
        );

        let new_locked_tokens = locked_token_mapper
            .nft_add_quantity(sft_nonce, new_locked_amount_attributes.token_amount);

        self.fees_from_penalty_unlocking().set(NonceAmountPair::new(
            new_locked_tokens.token_nonce,
            new_locked_tokens.amount,
        ));
    }

    fn send_fees_to_collector(&self) {
        // Send fees to FeeCollector SC
        let current_epoch = self.blockchain().get_block_epoch();
        let last_epoch_fee_sent_to_collector = self.last_epoch_fee_sent_to_collector().get();
        let next_send_epoch = last_epoch_fee_sent_to_collector + EPOCHS_PER_WEEK;

        if current_epoch < next_send_epoch {
            return;
        }

        let sc_address = self.fees_collector_address().get();
        let locked_token_id = self.locked_token().get_token_id();
        let nonce_amount_pair = self.fees_from_penalty_unlocking().get();

        self.fees_from_penalty_unlocking().clear();
        self.fees_collector_proxy_builder(sc_address)
            .deposit_swap_fees()
            .add_esdt_token_transfer(
                locked_token_id,
                nonce_amount_pair.nonce,
                nonce_amount_pair.amount,
            )
            .execute_on_dest_context_ignore_result();

        self.last_epoch_fee_sent_to_collector().set(current_epoch);
    }

    #[proxy]
    fn fees_collector_proxy_builder(
        &self,
        sc_address: ManagedAddress,
    ) -> fees_collector_proxy::Proxy<Self::Api>;

    #[view(getFeesBurnPercentage)]
    #[storage_mapper("feesBurnPercentage")]
    fn fees_burn_percentage(&self) -> SingleValueMapper<u16>;

    #[view(getFeesCollectorAddress)]
    #[storage_mapper("feesCollectorAddress")]
    fn fees_collector_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getFeesFromPenaltyUnlocking)]
    #[storage_mapper("feesFromPenaltyUnlocking")]
    fn fees_from_penalty_unlocking(&self) -> SingleValueMapper<NonceAmountPair<Self::Api>>;

    #[view(getLastEpochFeeSentToCollector)]
    #[storage_mapper("lastEpochFeeSentToCollector")]
    fn last_epoch_fee_sent_to_collector(&self) -> SingleValueMapper<Epoch>;
}
