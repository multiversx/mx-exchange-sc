elrond_wasm::imports!();

use common_structs::PaymentsVec;
use mergeable::{weighted_average, Mergeable};
use simple_lock::locked_token::LockedTokenAttributes;

use crate::{
    energy::Energy,
    penalty::{self},
};

#[derive(Clone)]
pub struct LockedAmountWeightAttributesPair<M: ManagedTypeApi> {
    pub token_amount: BigUint<M>,
    pub token_unlock_fee_percent: u64,
    pub attributes: LockedTokenAttributes<M>,
}

impl<M: ManagedTypeApi> Mergeable<M> for LockedAmountWeightAttributesPair<M> {
    fn can_merge_with(&self, other: &Self) -> bool {
        let same_token_id = self.attributes.original_token_id == other.attributes.original_token_id;
        let same_token_nonce =
            self.attributes.original_token_nonce == other.attributes.original_token_nonce;

        same_token_id && same_token_nonce
    }

    fn merge_with(&mut self, other: Self) {
        self.error_if_not_mergeable(&other);

        let unlock_fee: BigUint<M> = weighted_average(
            &BigUint::from(self.token_unlock_fee_percent),
            &self.token_amount.clone(),
            &BigUint::from(other.token_unlock_fee_percent),
            &other.token_amount.clone(),
        );

        self.token_amount += other.token_amount;
        self.token_unlock_fee_percent = unsafe { unlock_fee.to_u64().unwrap_unchecked() };
    }
}

#[elrond_wasm::module]
pub trait TokenMergingModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::pause::PauseModule
    + penalty::LocalPenaltyModule
    + crate::energy::EnergyModule
    + crate::events::EventsModule
    + crate::lock_options::LockOptionsModule
    + utils::UtilsModule
{
    // TODO: Only allow original caller arg for whitelisted addresses
    #[payable("*")]
    #[endpoint(mergeTokens)]
    fn merge_tokens_endpoint(
        &self,
        opt_original_caller: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment {
        self.require_not_paused();

        let actual_caller = self.blockchain().get_caller();

        let payments = self.get_non_empty_payments();

        let output_amount_attributes = self.merge_tokens(payments, opt_original_caller);

        let simulated_lock_payment = EgldOrEsdtTokenPayment::new(
            output_amount_attributes.attributes.original_token_id,
            output_amount_attributes.attributes.original_token_nonce,
            output_amount_attributes.token_amount,
        );
        let output_tokens = self.lock_and_send(
            &actual_caller,
            simulated_lock_payment,
            output_amount_attributes.attributes.unlock_epoch,
        );

        self.to_esdt_payment(output_tokens)
    }

    fn merge_tokens(
        self,
        mut payments: PaymentsVec<Self::Api>,
        opt_original_caller: OptionalValue<ManagedAddress>,
    ) -> LockedAmountWeightAttributesPair<Self::Api> {
        let locked_token_mapper = self.locked_token();
        let original_caller = self.dest_from_optional(opt_original_caller);
        let current_epoch = self.blockchain().get_block_epoch();
        let penalty_percentage_struct = self.penalty_percentage().get();

        locked_token_mapper.require_all_same_token(&payments);

        let first_payment = payments.get(0);
        payments.remove(0);

        self.update_energy(&original_caller, |energy: &mut Energy<Self::Api>| {
            let first_token_attributes: LockedTokenAttributes<Self::Api> =
                locked_token_mapper.get_token_attributes(first_payment.token_nonce);
            energy.update_after_unlock_any(
                &first_payment.amount,
                first_token_attributes.unlock_epoch,
                current_epoch,
            );

            locked_token_mapper.nft_burn(first_payment.token_nonce, &first_payment.amount);

            let lock_epochs_remaining = first_token_attributes.unlock_epoch - current_epoch;
            let mut output_pair = LockedAmountWeightAttributesPair {
                token_amount: first_payment.amount,
                token_unlock_fee_percent: self.calculate_penalty_percentage_full_unlock(
                    lock_epochs_remaining,
                    &penalty_percentage_struct,
                ),
                attributes: first_token_attributes,
            };

            for payment in &payments {
                let attributes: LockedTokenAttributes<Self::Api> =
                    locked_token_mapper.get_token_attributes(payment.token_nonce);
                energy.update_after_unlock_any(
                    &payment.amount,
                    attributes.unlock_epoch,
                    current_epoch,
                );

                locked_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

                let lock_epochs_remaining = attributes.unlock_epoch - current_epoch;

                let amount_attr_pair = LockedAmountWeightAttributesPair {
                    token_amount: payment.amount,
                    token_unlock_fee_percent: self.calculate_penalty_percentage_full_unlock(
                        lock_epochs_remaining,
                        &penalty_percentage_struct,
                    ),
                    attributes,
                };

                output_pair.merge_with(amount_attr_pair.clone());

                let penalty_percentage_struct = self.penalty_percentage().get();
                output_pair.attributes.unlock_epoch = self.calculate_epoch_from_penalty_percentage(
                    output_pair.token_unlock_fee_percent,
                    &penalty_percentage_struct,
                );
            }

            let normalized_unlock_epoch = self
                .unlock_epoch_to_start_of_month_upper_estimate(output_pair.attributes.unlock_epoch);
            output_pair.attributes.unlock_epoch = normalized_unlock_epoch;

            energy.add_after_token_lock(
                &output_pair.token_amount,
                output_pair.attributes.unlock_epoch,
                current_epoch,
            );

            output_pair
        })
    }
}
