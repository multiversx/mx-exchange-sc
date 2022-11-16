elrond_wasm::imports!();

use common_structs::{Epoch, Nonce};

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
pub trait FeesAccumulationModule:
    crate::fees_merging::FeesMergingModule
    + energy_factory::penalty::LocalPenaltyModule
    + energy_factory::lock_options::LockOptionsModule
{
    #[payable("*")]
    #[endpoint(depositFees)]
    fn deposit_fees(&self) {}

    fn burn_penalty(&self, token_id: TokenIdentifier, token_nonce: Nonce, fees_amount: &BigUint) {
        let fees_burn_percentage = self.fees_burn_percentage().get();
        let burn_amount = fees_amount * fees_burn_percentage as u64 / MAX_PENALTY_PERCENTAGE;
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
        let _: IgnoreValue = self
            .fees_collector_proxy_builder(sc_address)
            .deposit_swap_fees()
            .add_esdt_token_transfer(
                locked_token_id,
                nonce_amount_pair.nonce,
                nonce_amount_pair.amount,
            )
            .execute_on_dest_context();

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

    #[view(getLastEpochFeeSentToCollector)]
    #[storage_mapper("lastEpochFeeSentToCollector")]
    fn last_epoch_fee_sent_to_collector(&self) -> SingleValueMapper<Epoch>;
}
