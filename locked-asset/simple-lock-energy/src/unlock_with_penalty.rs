elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{Epoch, Nonce, NonceAmountPair, PaymentsVec};

use simple_lock::locked_token::LockedTokenAttributes;

use crate::{token_merging, penalty::{self, PenaltyPercentage}};

const MAX_PERCENTAGE: u16 = 10_000; // 100%
const MIN_EPOCHS_TO_REDUCE: Epoch = 1;
const EPOCHS_PER_WEEK: Epoch = 7;
static INVALID_PERCENTAGE_ERR_MSG: &[u8] = b"Invalid percentage value";

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
pub trait UnlockWithPenaltyModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::energy::EnergyModule
    + crate::lock_options::LockOptionsModule
    + crate::events::EventsModule
    + elrond_wasm_modules::pause::PauseModule
    + token_merging::TokenMergingModule
    + penalty::LocalPenaltyModule
    + utils::UtilsModule
{
    /// - min_penalty_percentage / max_penalty_percentage: The penalty for early unlock
    ///     of a token. A token locked for the max period, will have max_penalty_percentage penalty,
    ///     whereas one with 1 epoch left, will have min_penalty_percentage.
    ///     Penalty decreases linearly from max to min, based on the remaining locking period.
    ///     
    ///     Both are values between 0 and 10_000, where 10_000 is 100%.
    #[only_owner]
    #[endpoint(setPenaltyPercentage)]
    fn set_penalty_percentage(
        &self,
        first_threshold_penalty_percentage: u16,
        second_threshold_penalty_percentage: u16,
        third_threshold_penalty_percentage: u16,
    ) {
        let is_first_threshold_valid = first_threshold_penalty_percentage > 0
            && first_threshold_penalty_percentage < second_threshold_penalty_percentage;
        let is_second_threshold_valid =
            second_threshold_penalty_percentage < third_threshold_penalty_percentage;
        let is_third_threshold_valid = third_threshold_penalty_percentage < MAX_PERCENTAGE;

        require!(
            is_first_threshold_valid && is_second_threshold_valid && is_third_threshold_valid,
            INVALID_PERCENTAGE_ERR_MSG
        );

        self.penalty_percentage().set(&PenaltyPercentage {
            first_threshold: first_threshold_penalty_percentage,
            second_threshold: second_threshold_penalty_percentage,
            third_threshold: third_threshold_penalty_percentage,
        });
    }

    /// Sets the percentage of fees that are burned. The rest are sent to the fees collector.
    /// Value between 0 and 10_000. 0 is also accepted.
    #[only_owner]
    #[endpoint(setFeesBurnPercentage)]
    fn set_fees_burn_percentage(&self, percentage: u16) {
        require!(percentage <= MAX_PERCENTAGE, INVALID_PERCENTAGE_ERR_MSG);

        self.fees_burn_percentage().set(percentage);
    }

    #[only_owner]
    #[endpoint(setFeesCollectorAddress)]
    fn set_fees_collector_address(&self, sc_address: ManagedAddress) {
        self.require_sc_address(&sc_address);
        self.fees_collector_address().set(&sc_address);
    }

    /// Unlock a locked token instantly. This incures a penalty.
    /// The longer the remaining locking time, the bigger the penalty.
    #[payable("*")]
    #[endpoint(unlockEarly)]
    fn unlock_early(&self) -> EsdtTokenPayment {
        self.reduce_lock_period_common(None)
    }

    /// Reduce the locking period of a locked token. This incures a penalty.
    /// The longer the reduction, the bigger the penalty.
    /// epochs_to_reduce must be a multiple of 360 (i.e. 1 year)
    #[payable("*")]
    #[endpoint(reduceLockPeriod)]
    fn reduce_lock_period(&self, target_epoch_to_reduce: Epoch) -> EsdtTokenPayment {
        self.require_is_listed_lock_option(target_epoch_to_reduce);
        self.reduce_lock_period_common(Some(target_epoch_to_reduce))
    }

    fn reduce_lock_period_common(&self, opt_epochs_to_reduce: Option<Epoch>) -> EsdtTokenPayment {
        self.require_not_paused();

        let locked_token_mapper = self.locked_token();
        let payment = self.call_value().single_esdt();
        locked_token_mapper.require_same_token(&payment.token_identifier);

        let attributes: LockedTokenAttributes<Self::Api> =
            locked_token_mapper.get_token_attributes(payment.token_nonce);

        let target_epochs_to_reduce =
            self.resolve_opt_epochs_to_reduce(opt_epochs_to_reduce, attributes.unlock_epoch);

        let penalty_amount = self.calculate_penalty_amount(
            &payment.amount,
            target_epochs_to_reduce,
            attributes.unlock_epoch,
        );

        locked_token_mapper.nft_burn(payment.token_nonce, &(&payment.amount - &penalty_amount));

        let current_epoch = self.blockchain().get_block_epoch();
        let caller = self.blockchain().get_caller();

        let mut energy = self.get_updated_energy_entry_for_user(&caller);

        energy.deplete_after_early_unlock(&payment.amount, attributes.unlock_epoch, current_epoch);

        let mut unlocked_tokens = self.unlock_tokens_unchecked(payment.clone(), &attributes);
        let unlocked_token_id = unlocked_tokens.token_identifier.clone().unwrap_esdt();
        let new_unlock_epoch = current_epoch + target_epochs_to_reduce;

        if target_epochs_to_reduce == 0u64 {
            self.send().esdt_local_mint(
                &unlocked_token_id,
                0,
                &(&unlocked_tokens.amount - &penalty_amount),
            );
        }

        if penalty_amount > 0 {
            unlocked_tokens.amount -= &penalty_amount;
            require!(
                unlocked_tokens.amount > 0,
                "No tokens remaining after penalty is applied"
            );

            self.burn_penalty(
                locked_token_mapper.get_token_id(),
                payment.token_nonce,
                &penalty_amount,
            );
        }

        let output_payment = self.lock_and_send(&caller, unlocked_tokens, new_unlock_epoch);

        energy.add_after_token_lock(&output_payment.amount, new_unlock_epoch, current_epoch);

        self.set_energy_entry(&caller, energy);

        self.to_esdt_payment(output_payment)
    }

    fn resolve_opt_epochs_to_reduce(
        &self,
        opt_epochs_to_reduce: Option<Epoch>,
        original_unlock_epoch: Epoch,
    ) -> Epoch {
        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            original_unlock_epoch > current_epoch,
            "Token can be unlocked already"
        );

        let lock_epochs_remaining = original_unlock_epoch - current_epoch;
        match opt_epochs_to_reduce {
            Some(epochs_to_reduce) => {
                require!(
                    epochs_to_reduce >= MIN_EPOCHS_TO_REDUCE
                        && epochs_to_reduce <= lock_epochs_remaining,
                    "Invalid epochs to reduce"
                );
                epochs_to_reduce
            }
            None => 0u64,
        }
    }

    fn calculate_penalty_percentage_partial_unlock(
        &self,
        target_epoch_to_reduce: Epoch,
        lock_epochs_remaining: Epoch,
        penalty_percentage_struct: PenaltyPercentage,
    ) -> u64 {
        let penalty_percentage_full_unlock_current_epoch = self
            .calculate_penalty_percentage_full_unlock(
                lock_epochs_remaining,
                &penalty_percentage_struct,
            );
        let penalty_percentage_full_unlock_target_epoch = self
            .calculate_penalty_percentage_full_unlock(
                target_epoch_to_reduce,
                &penalty_percentage_struct,
            );

        (penalty_percentage_full_unlock_current_epoch - penalty_percentage_full_unlock_target_epoch)
            * MAX_PERCENTAGE as u64
            / (MAX_PERCENTAGE as u64 - penalty_percentage_full_unlock_target_epoch)
    }

    /// Calculates the penalty that would be incurred if token_amount tokens
    /// were to have their locking period reduce by epochs_to_reduce.
    /// target_epoch_to_reduce is one of 0, 360, 720, 1080 (0, 1, 2, 3 years)
    #[view(getPenaltyAmount)]
    fn calculate_penalty_amount(
        &self,
        token_amount: &BigUint,
        target_epoch_to_reduce: Epoch,
        current_unlock_epoch: Epoch,
    ) -> BigUint {
        self.require_is_listed_unlock_option(target_epoch_to_reduce);
        let current_epoch = self.blockchain().get_block_epoch();
        let lock_epochs_remaining = current_unlock_epoch - current_epoch;
        let penalty_percentage_struct = self.penalty_percentage().get();
        let full_unlock = target_epoch_to_reduce == 0;

        let penalty_percentage_unlock = if full_unlock {
            self.calculate_penalty_percentage_full_unlock(
                lock_epochs_remaining,
                &penalty_percentage_struct,
            )
        } else {
            self.calculate_penalty_percentage_partial_unlock(
                target_epoch_to_reduce,
                lock_epochs_remaining,
                penalty_percentage_struct,
            )
        };

        token_amount * penalty_percentage_unlock / MAX_PERCENTAGE as u64
    }

    fn burn_penalty(&self, token_id: TokenIdentifier, token_nonce: Nonce, fees_amount: &BigUint) {
        let fees_burn_percentage = self.fees_burn_percentage().get();
        let burn_amount = fees_amount * fees_burn_percentage as u64 / MAX_PERCENTAGE as u64;
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
                self.merge_fees_from_penalty(token_nonce, remaining_amount)
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

    #[endpoint(sendFeesToCollector)]
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
