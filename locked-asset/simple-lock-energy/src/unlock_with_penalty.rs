elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Epoch;
use simple_lock::locked_token::LockedTokenAttributes;

const MAX_PERCENTAGE: u16 = 10_000; // 100%
const MIN_EPOCHS_TO_REDUCE: Epoch = 1;

#[derive(TopEncode, TopDecode)]
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
    + crate::token_whitelist::TokenWhitelistModule
    + crate::util::UtilModule
    + crate::energy::EnergyModule
    + crate::migration::SimpleLockMigrationModule
    + crate::lock_options::LockOptionsModule
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
            "Invalid percentage value"
        );

        self.penalty_percentage().set(&PenaltyPercentage {
            min: min_penalty_percentage,
            max: max_penalty_percentage,
        });
    }

    #[payable("*")]
    #[endpoint(unlockEarly)]
    fn unlock_early(&self) -> EsdtTokenPayment {
        self.reduce_lock_period_common(None)
    }

    #[payable("*")]
    #[endpoint(reduceLockPeriod)]
    fn reduce_lock_period(&self, epochs_to_reduce: Epoch) -> EsdtTokenPayment {
        self.reduce_lock_period_common(Some(epochs_to_reduce))
    }

    fn reduce_lock_period_common(&self, opt_epochs_to_reduce: Option<Epoch>) -> EsdtTokenPayment {
        self.require_not_paused();

        let payment = self.call_value().single_esdt();
        self.require_is_new_token(payment.token_nonce);

        let attributes: LockedTokenAttributes<Self::Api> = self
            .locked_token()
            .get_token_attributes(payment.token_nonce);

        let epochs_to_reduce =
            self.resolve_opt_epochs_to_reduce(opt_epochs_to_reduce, attributes.unlock_epoch);
        let penalty_amount = self.calculate_penalty_amount(&payment.amount, epochs_to_reduce);
        let mut unlocked_tokens = self.unlock_tokens(payment);

        if penalty_amount > 0 {
            unlocked_tokens.amount -= &penalty_amount;
            require!(
                unlocked_tokens.amount > 0,
                "No tokens remaining after penalty is applied"
            );

            self.burn_penalty(&penalty_amount);
        }

        let caller = self.blockchain().get_caller();
        let new_unlock_epoch = attributes.unlock_epoch - epochs_to_reduce;
        let output_payment = self.lock_and_send(&caller, unlocked_tokens, new_unlock_epoch);

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
            Some(val) => {
                require!(
                    val >= MIN_EPOCHS_TO_REDUCE && val <= lock_epochs_remaining,
                    "Invalid epochs to reduce"
                );

                val
            }
            None => lock_epochs_remaining,
        }
    }

    /// linear decrease as epochs_to_reduce decreases
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

    // TODO: Burn x%, and rest send to fees collector
    fn burn_penalty(&self, _amount: &BigUint) {}

    #[proxy]
    fn fees_collector_proxy_builder(
        &self,
        sc_address: ManagedAddress,
    ) -> fees_collector_proxy::Proxy<Self::Api>;

    #[storage_mapper("penaltyPercentage")]
    fn penalty_percentage(&self) -> SingleValueMapper<PenaltyPercentage>;

    #[storage_mapper("feesBurnPercentage")]
    fn fees_burn_percentage(&self) -> SingleValueMapper<u16>;
}
