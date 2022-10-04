elrond_wasm::imports!();

use simple_lock::locked_token::LockedTokenAttributes;

use crate::energy::Energy;

static CANNOT_MERGE_ERR_MSG: &[u8] = b"Cannot merge";

pub trait Mergeable {
    fn can_merge_with(&self, other: &Self) -> bool;

    fn merge_with(&mut self, other: Self);
}

impl<M: ManagedTypeApi> Mergeable for EsdtTokenPayment<M> {
    fn can_merge_with(&self, other: &Self) -> bool {
        let same_token_id = self.token_identifier == other.token_identifier;
        let same_token_nonce = self.token_nonce == other.token_nonce;

        same_token_id && same_token_nonce
    }

    fn merge_with(&mut self, other: Self) {
        if !self.can_merge_with(&other) {
            M::error_api_impl().signal_error(CANNOT_MERGE_ERR_MSG);
        }

        self.amount += other.amount;
    }
}

pub struct LockedAmountAttributesPair<M: ManagedTypeApi> {
    pub token_amount: BigUint<M>,
    pub attributes: LockedTokenAttributes<M>,
}

impl<M: ManagedTypeApi> Mergeable for LockedAmountAttributesPair<M> {
    fn can_merge_with(&self, other: &Self) -> bool {
        let same_token_id = self.attributes.original_token_id == other.attributes.original_token_id;
        let same_token_nonce =
            self.attributes.original_token_nonce == other.attributes.original_token_nonce;

        same_token_id && same_token_nonce
    }

    fn merge_with(&mut self, other: Self) {
        if !self.can_merge_with(&other) {
            M::error_api_impl().signal_error(CANNOT_MERGE_ERR_MSG);
        }

        let first_unlock_epoch_weighted = &self.token_amount * self.attributes.unlock_epoch;
        let second_unlock_epoch_weighted = &other.token_amount * other.attributes.unlock_epoch;
        let total_weight = &self.token_amount + &other.token_amount;
        let new_unlock_epoch =
            (first_unlock_epoch_weighted + second_unlock_epoch_weighted) / total_weight;

        self.token_amount += other.token_amount;
        self.attributes.unlock_epoch = unsafe { new_unlock_epoch.to_u64().unwrap_unchecked() };
    }
}

#[elrond_wasm::module]
pub trait TokenMergingModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::token_attributes::TokenAttributesModule
    + crate::util::UtilModule
    + elrond_wasm_modules::pause::PauseModule
    + crate::energy::EnergyModule
    + crate::events::EventsModule
    + crate::lock_options::LockOptionsModule
{
    #[endpoint(mergeTokens)]
    fn merge_tokens(&self) -> EsdtTokenPayment {
        self.require_not_paused();

        let current_epoch = self.blockchain().get_block_epoch();
        let caller = self.blockchain().get_caller();
        let locked_token_mapper = self.locked_token();

        let mut payments = self.get_non_empty_payments();
        locked_token_mapper.require_all_same_token(&payments);

        let first_payment = payments.get(0);
        payments.remove(0);

        let output_amount_attributes =
            self.update_energy(&caller, |energy: &mut Energy<Self::Api>| {
                let first_token_attributes: LockedTokenAttributes<Self::Api> =
                    locked_token_mapper.get_token_attributes(first_payment.token_nonce);
                energy.update_after_unlock_any(
                    &first_payment.amount,
                    first_token_attributes.unlock_epoch,
                    current_epoch,
                );

                locked_token_mapper.nft_burn(first_payment.token_nonce, &first_payment.amount);

                let mut output_pair = LockedAmountAttributesPair {
                    token_amount: first_payment.amount,
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

                    let amount_attr_pair = LockedAmountAttributesPair {
                        token_amount: payment.amount,
                        attributes,
                    };
                    output_pair.merge_with(amount_attr_pair);
                }

                energy.add_after_token_lock(
                    &output_pair.token_amount,
                    output_pair.attributes.unlock_epoch,
                    current_epoch,
                );

                output_pair
            });

        let normalized_unlock_epoch = self.unlock_epoch_to_start_of_month_upper_estimate(
            output_amount_attributes.attributes.unlock_epoch,
        );
        let simulated_lock_payment = EgldOrEsdtTokenPayment::new(
            output_amount_attributes.attributes.original_token_id,
            output_amount_attributes.attributes.original_token_nonce,
            output_amount_attributes.token_amount,
        );
        let output_tokens =
            self.lock_and_send(&caller, simulated_lock_payment, normalized_unlock_epoch);

        self.to_esdt_payment(output_tokens)
    }
}
