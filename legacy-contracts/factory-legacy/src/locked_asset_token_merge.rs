multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::*;

use crate::attr_ex_helper::{self, PRECISION_EX_INCREASE};

use super::locked_asset;
use super::locked_asset::{
    EpochAmountPair, LockedTokenEx, DOUBLE_MAX_MILESTONES_IN_SCHEDULE, MAX_MILESTONES_IN_SCHEDULE,
    ONE_MILLION, PERCENTAGE_TOTAL_EX,
};

#[multiversx_sc::module]
pub trait LockedAssetTokenMergeModule:
    locked_asset::LockedAssetModule
    + token_merge_helper::TokenMergeHelperModule
    + attr_ex_helper::AttrExHelper
{
    fn burn_tokens_from_payments(&self, payments: &ManagedVec<EsdtTokenPayment>) {
        for entry in payments {
            self.send()
                .esdt_local_burn(&entry.token_identifier, entry.token_nonce, &entry.amount);
        }
    }

    fn get_merged_locked_asset_token_amount_and_attributes(
        &self,
        payments: &ManagedVec<EsdtTokenPayment>,
    ) -> (BigUint, LockedAssetTokenAttributesEx<Self::Api>) {
        require!(!payments.is_empty(), "Cannot merge with 0 tokens");

        let mut tokens = ManagedVec::new();
        let mut sum_amount = BigUint::zero();
        let locked_asset_token_id = self.locked_asset_token_id().get();
        let attr_ex_activation = self.extended_attributes_activation_nonce().get();
        for entry in payments {
            require!(
                entry.token_identifier == locked_asset_token_id,
                "Bad token id"
            );

            tokens.push(LockedTokenEx {
                token_amount: EsdtTokenPayment::new(
                    entry.token_identifier.clone(),
                    entry.token_nonce,
                    entry.amount.clone(),
                ),
                attributes: self.get_attributes_ex(
                    &entry.token_identifier,
                    entry.token_nonce,
                    attr_ex_activation,
                ),
            });
            sum_amount += &entry.amount;
        }

        if tokens.len() == 1 {
            let token_0 = tokens.get(0);
            return (
                token_0.token_amount.amount.clone(),
                token_0.attributes.clone(),
            );
        }

        let attrs = LockedAssetTokenAttributesEx {
            unlock_schedule: self.aggregated_unlock_schedule(&tokens),
            is_merged: true,
        };

        (sum_amount, attrs)
    }

    fn calculate_new_unlock_milestones(
        &self,
        unlock_epoch_amount_merged: &ArrayVec<
            EpochAmountPair<Self::Api>,
            DOUBLE_MAX_MILESTONES_IN_SCHEDULE,
        >,
        amount_total: &BigUint,
    ) -> ManagedVec<UnlockMilestoneEx> {
        let mut unlock_milestones_merged =
            ArrayVec::<UnlockMilestoneEx, MAX_MILESTONES_IN_SCHEDULE>::new();

        for el in unlock_epoch_amount_merged.iter() {
            let unlock_percent = &(&el.amount * PRECISION_EX_INCREASE * ONE_MILLION) / amount_total;

            // Accumulate even the percents of 0
            unlock_milestones_merged.push(UnlockMilestoneEx {
                unlock_epoch: el.epoch,
                unlock_percent: unlock_percent.to_u64().unwrap(),
            })
        }

        self.distribute_leftover(&mut unlock_milestones_merged);
        self.get_non_zero_percent_milestones_as_vec(&unlock_milestones_merged)
    }

    fn aggregated_unlock_schedule(
        &self,
        tokens: &ManagedVec<LockedTokenEx<Self::Api>>,
    ) -> UnlockScheduleEx<Self::Api> {
        let mut array =
            ArrayVec::<EpochAmountPair<Self::Api>, DOUBLE_MAX_MILESTONES_IN_SCHEDULE>::new();

        let mut sum = BigUint::zero();
        for locked_token in tokens.iter() {
            for milestone in locked_token
                .attributes
                .unlock_schedule
                .unlock_milestones
                .iter()
            {
                require!(
                    array.len() < DOUBLE_MAX_MILESTONES_IN_SCHEDULE,
                    "too many unlock milestones"
                );
                array.push(EpochAmountPair {
                    epoch: milestone.unlock_epoch,
                    amount: self.rule_of_three(
                        &BigUint::from(milestone.unlock_percent),
                        &BigUint::from(PERCENTAGE_TOTAL_EX),
                        &locked_token.token_amount.amount,
                    ),
                });
            }
            sum += &locked_token.token_amount.amount;
        }
        array.sort_unstable_by(|a, b| a.epoch.cmp(&b.epoch));

        let default = EpochAmountPair {
            epoch: 0u64,
            amount: BigUint::zero(),
        };
        let mut unlock_epoch_amount_merged =
            ArrayVec::<EpochAmountPair<Self::Api>, DOUBLE_MAX_MILESTONES_IN_SCHEDULE>::new();
        for elem in array.iter() {
            let last = unlock_epoch_amount_merged.last().unwrap_or(&default);

            if elem.epoch == last.epoch || elem.epoch == last.epoch + 1 {
                let new_elem = EpochAmountPair {
                    epoch: elem.epoch,
                    amount: &last.amount + &elem.amount,
                };
                unlock_epoch_amount_merged.pop();
                unlock_epoch_amount_merged.push(new_elem);
            } else {
                unlock_epoch_amount_merged.push(elem.clone());
            }
        }
        require!(sum != 0u64, "Sum cannot be zero");
        require!(
            unlock_epoch_amount_merged.len() < MAX_MILESTONES_IN_SCHEDULE,
            "Too many milestones"
        );
        require!(!unlock_epoch_amount_merged.is_empty(), "Empty milestones");

        let new_unlock_milestones =
            self.calculate_new_unlock_milestones(&unlock_epoch_amount_merged, &sum);

        UnlockScheduleEx {
            unlock_milestones: new_unlock_milestones,
        }
    }
}
