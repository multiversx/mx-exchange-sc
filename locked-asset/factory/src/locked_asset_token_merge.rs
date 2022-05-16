elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::*;

use crate::attr_ex_helper::{self, PRECISION_EX_INCREASE};

use super::locked_asset;
use super::locked_asset::{
    EpochAmountPair, LockedTokenEx, DOUBLE_MAX_MILESTONES_IN_SCHEDULE, MAX_MILESTONES_IN_SCHEDULE,
    ONE_MILLION, PERCENTAGE_TOTAL_EX,
};

#[elrond_wasm::module]
pub trait LockedAssetTokenMergeModule:
    locked_asset::LockedAssetModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
    + attr_ex_helper::AttrExHelper
{
    #[payable("*")]
    #[endpoint(mergeLockedAssetTokens)]
    fn merge_locked_asset_tokens(&self) -> EsdtTokenPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payments_vec = self.call_value().all_esdt_transfers();
        require!(!payments_vec.is_empty(), "Empty payment vec");
        let payments_iter = payments_vec.iter();

        let (amount, attrs) =
            self.get_merged_locked_asset_token_amount_and_attributes(payments_iter.clone());
        let locked_asset_token = self.locked_asset_token_id().get();

        self.burn_tokens_from_payments(payments_iter);

        let new_nonce = self
            .send()
            .esdt_nft_create_compact(&locked_asset_token, &amount, &attrs);
        self.send()
            .direct(&caller, &locked_asset_token, new_nonce, &amount, &[]);

        EsdtTokenPayment::new(locked_asset_token, new_nonce, amount)
    }

    fn burn_tokens_from_payments(
        &self,
        payments: ManagedVecRefIterator<Self::Api, EsdtTokenPayment<Self::Api>>,
    ) {
        for entry in payments {
            self.send()
                .esdt_local_burn(&entry.token_identifier, entry.token_nonce, &entry.amount);
        }
    }

    fn get_merged_locked_asset_token_amount_and_attributes(
        &self,
        payments: ManagedVecRefIterator<Self::Api, EsdtTokenPayment<Self::Api>>,
    ) -> (BigUint, LockedAssetTokenAttributesEx<Self::Api>) {
        require!(!payments.is_empty(), "Cannot merge with 0 tokens");

        let mut tokens = ManagedVec::new();
        let mut sum_amount = BigUint::zero();
        let locked_asset_token_id = self.locked_asset_token_id().get();

        for entry in payments {
            require!(
                entry.token_identifier == locked_asset_token_id,
                "Bad token id"
            );

            sum_amount += &entry.amount;

            let attributes = self.get_attributes_ex(&entry.token_identifier, entry.token_nonce);
            let payment =
                EsdtTokenPayment::new(entry.token_identifier, entry.token_nonce, entry.amount);
            tokens.push(LockedTokenEx {
                token_amount: payment,
                attributes,
            });
        }

        if tokens.len() == 1 {
            let token_0 = tokens.get(0);
            return (token_0.token_amount.amount, token_0.attributes);
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
