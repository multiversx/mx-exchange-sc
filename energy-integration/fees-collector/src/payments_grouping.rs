use common_types::PaymentsVec;
use weekly_rewards_splitting::USER_MAX_CLAIM_WEEKS;

use core::ops::Deref;

elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait PaymentsGroupingModule: crate::config::ConfigModule + utils::UtilsModule {
    fn group_payments(
        &self,
        mut claim_payments: ArrayVec<PaymentsVec<Self::Api>, USER_MAX_CLAIM_WEEKS>,
    ) -> PaymentsVec<Self::Api> {
        if claim_payments.is_empty() {
            return PaymentsVec::new();
        }
        if claim_payments.len() == 1 {
            return unsafe { claim_payments.get_unchecked(0).clone() };
        }

        let mut total_locked_rewards = BigUint::zero();
        let locked_token_id = self.locked_token_id().get();
        for vec in claim_payments.iter_mut() {
            let opt_locked_rewards = self.get_and_remove_locked_token_rewards(vec);
            if let Some(locked_rewards) = opt_locked_rewards {
                require!(
                    locked_rewards.token_identifier == locked_token_id,
                    "Invalid locked rewards"
                );

                total_locked_rewards += locked_rewards.amount;
            }
        }

        let mut merged_payments = PaymentsVec::new();
        let all_tokens = self.all_tokens().get();
        let empty_buffer = ManagedBuffer::new();
        for (i, token_id) in all_tokens.iter().enumerate() {
            let mut output_token_id = token_id.deref().clone();
            let mut total_for_token = BigUint::zero();
            for vec in &claim_payments {
                if vec.is_empty() {
                    continue;
                }

                if output_token_id.as_managed_buffer() == &empty_buffer {
                    let opt_token_id_in_vec = self.get_token_id_at_index(vec, i);
                    if let Some(token_id_in_vec) = opt_token_id_in_vec {
                        output_token_id = token_id_in_vec;
                    }
                }

                total_for_token += self.get_token_amount_at_index(vec, i);
            }

            if total_for_token > 0 && output_token_id.as_managed_buffer() != &empty_buffer {
                merged_payments.push(EsdtTokenPayment::new(output_token_id, 0, total_for_token));
            }
        }

        merged_payments.push(EsdtTokenPayment::new(
            locked_token_id,
            0,
            total_locked_rewards,
        ));

        merged_payments
    }

    /// Locked token rewards are always at the last index
    fn get_and_remove_locked_token_rewards(
        &self,
        payments: &mut PaymentsVec<Self::Api>,
    ) -> Option<EsdtTokenPayment> {
        if payments.is_empty() {
            return None;
        }

        let last_item_index = payments.len() - 1;
        let result = payments.get(last_item_index);
        payments.remove(last_item_index);

        Some(result)
    }

    fn get_token_id_at_index(
        &self,
        payments: &PaymentsVec<Self::Api>,
        index: usize,
    ) -> Option<TokenIdentifier> {
        if index >= payments.len() {
            return None;
        }

        let payment = payments.get(index);
        Some(payment.token_identifier)
    }

    fn get_token_amount_at_index(
        &self,
        payments: &PaymentsVec<Self::Api>,
        index: usize,
    ) -> BigUint {
        if index >= payments.len() {
            return BigUint::zero();
        }

        let payment = payments.get(index);
        payment.amount
    }
}
