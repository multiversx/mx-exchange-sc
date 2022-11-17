elrond_wasm::imports!();

use common_structs::PaymentsVec;
use math::{safe_sub, weighted_average};
use mergeable::Mergeable;
use simple_lock::locked_token::LockedTokenAttributes;
use unwrappable::Unwrappable;

use crate::{energy::Energy, unlock_with_penalty::TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG};

#[derive(Clone)]
pub struct LockedAmountWeightAttributesPair<'a, Sc>
where
    Sc: crate::penalty::LocalPenaltyModule,
{
    pub sc_ref: &'a Sc,
    pub token_amount: BigUint<Sc::Api>,
    pub token_unlock_fee_percent: u64,
    pub attributes: LockedTokenAttributes<Sc::Api>,
}

impl<'a, Sc> LockedAmountWeightAttributesPair<'a, Sc>
where
    Sc: crate::penalty::LocalPenaltyModule,
{
    pub fn new(
        sc_ref: &'a Sc,
        token_amount: BigUint<Sc::Api>,
        attributes: LockedTokenAttributes<Sc::Api>,
    ) -> Self {
        let current_epoch = sc_ref.blockchain().get_block_epoch();
        let lock_epochs_remaining = safe_sub(attributes.unlock_epoch, current_epoch);
        let token_unlock_fee_percent =
            sc_ref.calculate_penalty_percentage_full_unlock(lock_epochs_remaining);

        Self {
            sc_ref,
            token_amount,
            token_unlock_fee_percent,
            attributes,
        }
    }
}

impl<'a, Sc> Mergeable<Sc::Api> for LockedAmountWeightAttributesPair<'a, Sc>
where
    Sc: crate::penalty::LocalPenaltyModule,
{
    fn can_merge_with(&self, other: &Self) -> bool {
        let same_token_id = self.attributes.original_token_id == other.attributes.original_token_id;
        let same_token_nonce =
            self.attributes.original_token_nonce == other.attributes.original_token_nonce;

        same_token_id && same_token_nonce
    }

    fn merge_with(&mut self, other: Self) {
        self.error_if_not_mergeable(&other);

        let unlock_fee = weighted_average(
            BigUint::from(self.token_unlock_fee_percent),
            self.token_amount.clone(),
            BigUint::from(other.token_unlock_fee_percent),
            other.token_amount.clone(),
        );

        self.token_amount += other.token_amount;
        self.token_unlock_fee_percent = unlock_fee.to_u64().unwrap_or_panic::<Sc::Api>();

        let lock_epochs = self
            .sc_ref
            .calculate_lock_epochs_from_penalty_percentage(self.token_unlock_fee_percent);
        let current_epoch = self.sc_ref.blockchain().get_block_epoch();
        self.attributes.unlock_epoch = current_epoch + lock_epochs;
    }
}

#[elrond_wasm::module]
pub trait TokenMergingModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::pause::PauseModule
    + crate::penalty::LocalPenaltyModule
    + crate::energy::EnergyModule
    + crate::events::EventsModule
    + crate::lock_options::LockOptionsModule
    + utils::UtilsModule
    + sc_whitelist_module::SCWhitelistModule
{
    #[payable("*")]
    #[endpoint(mergeTokens)]
    fn merge_tokens_endpoint(
        &self,
        opt_original_caller: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment {
        self.require_not_paused();

        let payments = self.get_non_empty_payments();
        let caller = self.blockchain().get_caller();
        let original_caller = self.get_orig_caller_from_opt(&caller, opt_original_caller);

        let output_amount_attributes = self.update_energy(&original_caller, |energy| {
            self.merge_tokens(payments, energy)
        });
        let simulated_lock_payment = EgldOrEsdtTokenPayment::new(
            output_amount_attributes.attributes.original_token_id,
            output_amount_attributes.attributes.original_token_nonce,
            output_amount_attributes.token_amount,
        );
        let output_tokens = self.lock_and_send(
            &caller,
            simulated_lock_payment,
            output_amount_attributes.attributes.unlock_epoch,
        );

        self.to_esdt_payment(output_tokens)
    }

    fn merge_tokens(
        self,
        mut payments: PaymentsVec<Self::Api>,
        energy: &mut Energy<Self::Api>,
    ) -> LockedAmountWeightAttributesPair<Self> {
        let locked_token_mapper = self.locked_token();
        locked_token_mapper.require_all_same_token(&payments);

        let first_payment = payments.get(0);
        payments.remove(0);

        let current_epoch = self.blockchain().get_block_epoch();
        let first_token_attributes: LockedTokenAttributes<Self::Api> =
            locked_token_mapper.get_token_attributes(first_payment.token_nonce);
        require!(
            first_token_attributes.unlock_epoch > current_epoch,
            TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG
        );

        energy.update_after_unlock_any(
            &first_payment.amount,
            first_token_attributes.unlock_epoch,
            current_epoch,
        );

        locked_token_mapper.nft_burn(first_payment.token_nonce, &first_payment.amount);

        let mut output_pair = LockedAmountWeightAttributesPair::new(
            self,
            first_payment.amount,
            first_token_attributes,
        );
        for payment in &payments {
            let attributes: LockedTokenAttributes<Self::Api> =
                locked_token_mapper.get_token_attributes(payment.token_nonce);
            require!(
                attributes.unlock_epoch > current_epoch,
                TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG
            );

            energy.update_after_unlock_any(&payment.amount, attributes.unlock_epoch, current_epoch);

            locked_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

            let amount_attr_pair =
                LockedAmountWeightAttributesPair::new(self, payment.amount, attributes);
            output_pair.merge_with(amount_attr_pair);
        }

        let normalized_unlock_epoch =
            self.unlock_epoch_to_start_of_month_upper_estimate(output_pair.attributes.unlock_epoch);
        output_pair.attributes.unlock_epoch = normalized_unlock_epoch;

        energy.add_after_token_lock(
            &output_pair.token_amount,
            output_pair.attributes.unlock_epoch,
            current_epoch,
        );

        output_pair
    }
}
