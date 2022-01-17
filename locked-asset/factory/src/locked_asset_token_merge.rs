elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::*;

use super::locked_asset;
use super::locked_asset::{
    EpochAmountPair, LockedToken, DOUBLE_MAX_MILESTONES_IN_SCHEDULE, MAX_MILESTONES_IN_SCHEDULE,
    ONE_MILLION, PERCENTAGE_TOTAL,
};

#[elrond_wasm::module]
pub trait LockedAssetTokenMergeModule:
    locked_asset::LockedAssetModule + token_send::TokenSendModule + token_merge::TokenMergeModule
{
    #[payable("*")]
    #[endpoint(mergeLockedAssetTokens)]
    fn merge_locked_asset_tokens(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<EsdtTokenPayment<Self::Api>> {
        let caller = self.blockchain().get_caller();
        let payments_vec = self.get_all_payments_managed_vec();
        require!(!payments_vec.is_empty(), "Empty payment vec");
        let payments_iter = payments_vec.iter();

        let (amount, attrs) =
            self.get_merged_locked_asset_token_amount_and_attributes(payments_iter.clone())?;
        let locked_asset_token = self.locked_asset_token_id().get();

        self.burn_tokens_from_payments(payments_iter);

        let new_nonce = self.nft_create_tokens(&locked_asset_token, &amount, &attrs);
        self.transfer_execute_custom(
            &caller,
            &locked_asset_token,
            new_nonce,
            &amount,
            &opt_accept_funds_func,
        )?;

        Ok(self.create_payment(&locked_asset_token, new_nonce, &amount))
    }

    fn burn_tokens_from_payments(&self, payments: ManagedVecIterator<EsdtTokenPayment<Self::Api>>) {
        for entry in payments {
            self.send()
                .esdt_local_burn(&entry.token_identifier, entry.token_nonce, &entry.amount);
        }
    }

    fn get_merged_locked_asset_token_amount_and_attributes(
        &self,
        payments: ManagedVecIterator<EsdtTokenPayment<Self::Api>>,
    ) -> SCResult<(BigUint, LockedAssetTokenAttributes<Self::Api>)> {
        require!(!payments.is_empty(), "Cannot merge with 0 tokens");

        let mut tokens = ManagedVec::new();
        let mut sum_amount = BigUint::zero();
        let locked_asset_token_id = self.locked_asset_token_id().get();

        for entry in payments {
            require!(
                entry.token_identifier == locked_asset_token_id,
                "Bad token id"
            );

            tokens.push(LockedToken {
                token_amount: self.create_payment(
                    &entry.token_identifier,
                    entry.token_nonce,
                    &entry.amount,
                ),
                attributes: self.get_attributes(&entry.token_identifier, entry.token_nonce)?,
            });
            sum_amount += &entry.amount;
        }

        if tokens.len() == 1 {
            let token_0 = tokens.get(0).unwrap();
            return Ok((
                token_0.token_amount.amount.clone(),
                token_0.attributes.clone(),
            ));
        }

        let attrs = LockedAssetTokenAttributes {
            unlock_schedule: self.aggregated_unlock_schedule(&tokens)?,
            is_merged: true,
        };

        Ok((sum_amount, attrs))
    }

    fn calculate_new_unlock_milestones(
        &self,
        unlock_epoch_amount_merged: &ArrayVec<
            EpochAmountPair<Self::Api>,
            DOUBLE_MAX_MILESTONES_IN_SCHEDULE,
        >,
        amount_total: &BigUint,
    ) -> ManagedVec<UnlockMilestone> {
        let mut unlock_milestones_merged =
            ArrayVec::<UnlockMilestoneExtended, MAX_MILESTONES_IN_SCHEDULE>::new();

        for el in unlock_epoch_amount_merged.iter() {
            let unlock_percent = &(&el.amount * ONE_MILLION) / amount_total;

            //Accumulate even the percents of 0
            unlock_milestones_merged.push(UnlockMilestoneExtended {
                unlock_epoch: el.epoch,
                unlock_percent: unlock_percent.to_u64().unwrap(),
            })
        }

        self.distribute_leftover(&mut unlock_milestones_merged);
        self.get_non_zero_percent_milestones_as_vec(&unlock_milestones_merged)
    }

    fn aggregated_unlock_schedule(
        &self,
        tokens: &ManagedVec<LockedToken<Self::Api>>,
    ) -> SCResult<UnlockSchedule<Self::Api>> {
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
                        &BigUint::from(milestone.unlock_percent as u64),
                        &BigUint::from(PERCENTAGE_TOTAL as u64),
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

            if elem.epoch == last.epoch {
                let new_elem = EpochAmountPair {
                    epoch: last.epoch,
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

        Ok(UnlockSchedule {
            unlock_milestones: new_unlock_milestones,
        })
    }
}
