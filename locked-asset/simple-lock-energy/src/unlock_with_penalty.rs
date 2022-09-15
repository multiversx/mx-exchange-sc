elrond_wasm::imports!();

use common_structs::Epoch;
use simple_lock::locked_token::LockedTokenAttributes;

const MIN_PERCENTAGE: u64 = 100; // 1%
const MAX_PERCENTAGE: u64 = 10_000; // 100%
const MIN_EPOCHS_TO_REDUCE: Epoch = 1;

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
    /// The penalty for early unlock of a token locked with max period.
    /// Value between 100 and 10_000, where 100 is 1% and 10_000 is 100%.
    /// Penalty decreases linearly with the locking period.
    #[only_owner]
    #[endpoint(setMaxPenaltyPercentage)]
    fn set_max_penalty_percentage(&self, new_value: u64) {
        require!(
            new_value >= MIN_PERCENTAGE && new_value <= MAX_PERCENTAGE,
            "Invalid percentage value"
        );

        self.max_penalty_percentage().set(new_value);
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
    /// starting from max_penalty_percentage, all the way down to MIN_PERCENTAGE
    #[view(getPenaltyAmount)]
    fn calculate_penalty_amount(&self, token_amount: &BigUint, epochs_to_reduce: Epoch) -> BigUint {
        let max_penalty_percentage = self.max_penalty_percentage().get();
        let max_lock_option = self.max_lock_option().get();

        let penalty_percentage = MIN_PERCENTAGE
            + (max_penalty_percentage - MIN_PERCENTAGE) * epochs_to_reduce / max_lock_option;

        token_amount * penalty_percentage / MAX_PERCENTAGE
    }

    // TODO: Burn x%, and rest send to fees collector
    fn burn_penalty(&self, _amount: &BigUint) {}

    #[storage_mapper("maxPenaltyPercentage")]
    fn max_penalty_percentage(&self) -> SingleValueMapper<u64>;
}
