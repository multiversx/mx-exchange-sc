elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Epoch;
use simple_lock::locked_token::LockedTokenAttributes;

use crate::lock_options::EPOCHS_PER_MONTH;

const MAX_PERCENTAGE: u16 = 10_000; // 100%
const MIN_EPOCHS_TO_REDUCE: Epoch = 1;
static INVALID_PERCENTAGE_ERR_MSG: &[u8] = b"Invalid percentage value";

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct PenaltyPercentage {
    pub min: u16,
    pub max: u16,
}

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
    + crate::util::UtilModule
    + crate::energy::EnergyModule
    + crate::lock_options::LockOptionsModule
    + crate::old_token_nonces::OldTokenNonces
    + crate::events::EventsModule
    + elrond_wasm_modules::pause::PauseModule
{
    /// - min_penalty_percentage / max_penalty_percentage: The penalty for early unlock
    ///     of a token. A token locked for the max period, will have max_penalty_percentage penalty,
    ///     whereas one with 1 epoch left, will have min_penalty_percentage.
    ///     Penalty decreases linearly from max to min, based on the remaining locking period.
    ///     
    ///     Both are values between 0 and 10_000, where 10_000 is 100%.
    #[only_owner]
    #[endpoint(setPenaltyPercentage)]
    fn set_penalty_percentage(&self, min_penalty_percentage: u16, max_penalty_percentage: u16) {
        let is_min_valid = min_penalty_percentage > 0 && min_penalty_percentage <= MAX_PERCENTAGE;
        let is_max_valid = max_penalty_percentage > 0 && max_penalty_percentage <= MAX_PERCENTAGE;
        let correct_order = min_penalty_percentage <= max_penalty_percentage;
        require!(
            is_min_valid && is_max_valid && correct_order,
            INVALID_PERCENTAGE_ERR_MSG
        );

        self.penalty_percentage().set(&PenaltyPercentage {
            min: min_penalty_percentage,
            max: max_penalty_percentage,
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
        require!(
            !sc_address.is_zero() && self.blockchain().is_smart_contract(&sc_address),
            "Invalid SC address"
        );

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
    /// epochs_to_reduce must be a multiple of 30 (i.e. 1 month)
    #[payable("*")]
    #[endpoint(reduceLockPeriod)]
    fn reduce_lock_period(&self, epochs_to_reduce: Epoch) -> EsdtTokenPayment {
        require!(
            epochs_to_reduce % EPOCHS_PER_MONTH == 0,
            "May only reduce by multiples of months (30 epochs)"
        );

        self.reduce_lock_period_common(Some(epochs_to_reduce))
    }

    fn reduce_lock_period_common(&self, opt_epochs_to_reduce: Option<Epoch>) -> EsdtTokenPayment {
        self.require_not_paused();

        let payment = self.call_value().single_esdt();
        let attributes: LockedTokenAttributes<Self::Api> = self
            .locked_token()
            .get_token_attributes(payment.token_nonce);

        let epochs_to_reduce =
            self.resolve_opt_epochs_to_reduce(opt_epochs_to_reduce, attributes.unlock_epoch);
        let penalty_amount = self.calculate_penalty_amount(&payment.amount, epochs_to_reduce);

        let current_epoch = self.blockchain().get_block_epoch();
        let caller = self.blockchain().get_caller();

        let mut energy = self.get_updated_energy_entry_for_user(&caller, current_epoch);
        energy.deplete_after_early_unlock(&payment.amount, attributes.unlock_epoch, current_epoch);

        let mut unlocked_tokens = self.unlock_tokens_unchecked(payment, &attributes);
        if penalty_amount > 0 {
            unlocked_tokens.amount -= &penalty_amount;
            require!(
                unlocked_tokens.amount > 0,
                "No tokens remaining after penalty is applied"
            );

            let fees_token_id = unlocked_tokens.token_identifier.clone().unwrap_esdt();
            self.burn_penalty(fees_token_id, &penalty_amount);
        }

        let new_unlock_epoch =
            self.unlock_epoch_to_start_of_month(attributes.unlock_epoch - epochs_to_reduce);
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
            None => lock_epochs_remaining,
        }
    }

    /// Calculates the penalty that would be incurred if token_amount tokens
    /// were to have their locking period reduce by epochs_to_reduce.
    ///
    /// Linear decrease as epochs_to_reduce decreases
    /// starting from max penalty_percentage, all the way down to min
    #[view(getPenaltyAmount)]
    fn calculate_penalty_amount(&self, token_amount: &BigUint, epochs_to_reduce: Epoch) -> BigUint {
        let penalty_percentage = self.penalty_percentage().get();
        let min_penalty = penalty_percentage.min as u64;
        let max_penalty = penalty_percentage.max as u64;
        let max_lock_option = self.max_lock_option().get();

        let penalty_percentage =
            min_penalty + (max_penalty - min_penalty) * epochs_to_reduce / max_lock_option;

        token_amount * penalty_percentage / MAX_PERCENTAGE as u64
    }

    fn burn_penalty(&self, token_id: TokenIdentifier, fees_amount: &BigUint) {
        let fees_burn_percentage = self.fees_burn_percentage().get();
        let burn_amount = fees_amount * fees_burn_percentage as u64 / MAX_PERCENTAGE as u64;
        let remaining_amount = fees_amount - &burn_amount;

        if burn_amount > 0 {
            self.send().esdt_local_burn(&token_id, 0, &burn_amount);
        }
        if remaining_amount > 0 {
            self.send_fees_to_collector(token_id, remaining_amount);
        }
    }

    fn send_fees_to_collector(&self, token_id: TokenIdentifier, amount: BigUint) {
        let sc_address = self.fees_collector_address().get();
        self.fees_collector_proxy_builder(sc_address)
            .deposit_swap_fees()
            .add_esdt_token_transfer(token_id, 0, amount)
            .execute_on_dest_context_ignore_result();
    }

    #[proxy]
    fn fees_collector_proxy_builder(
        &self,
        sc_address: ManagedAddress,
    ) -> fees_collector_proxy::Proxy<Self::Api>;

    #[view(getPenaltyPercentage)]
    #[storage_mapper("penaltyPercentage")]
    fn penalty_percentage(&self) -> SingleValueMapper<PenaltyPercentage>;

    #[view(getFeesBurnPercentage)]
    #[storage_mapper("feesBurnPercentage")]
    fn fees_burn_percentage(&self) -> SingleValueMapper<u16>;

    #[view(getFeesCollectorAddress)]
    #[storage_mapper("feesCollectorAddress")]
    fn fees_collector_address(&self) -> SingleValueMapper<ManagedAddress>;
}
