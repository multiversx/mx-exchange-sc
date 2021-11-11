elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{LockedAssetTokenAttributes, UnlockMilestone, UnlockSchedule};

use super::locked_asset;
use super::locked_asset::PERCENTAGE_TOTAL;

const MAX_MILESTONES_IN_SCHEDULE: usize = 64;

pub struct LockedToken<M: ManagedTypeApi> {
    pub token_amount: EsdtTokenPayment<M>,
    pub attributes: LockedAssetTokenAttributes,
}

#[elrond_wasm::module]
pub trait LockedAssetTokenMergeModule:
    locked_asset::LockedAssetModule
    + token_supply::TokenSupplyModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
{
    #[payable("*")]
    #[endpoint(mergeLockedAssetTokens)]
    fn merge_locked_asset_tokens(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<EsdtTokenPayment<Self::Api>> {
        let caller = self.blockchain().get_caller();
        let payments = self.get_all_payments_managed_vec();
        require!(!payments.is_empty(), "Empty payment vec");

        let (amount, attrs) =
            self.get_merged_locked_asset_token_amount_and_attributes(&payments)?;
        let locked_asset_token = self.locked_asset_token_id().get();

        self.burn_tokens_from_payments(&payments);

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

    fn burn_tokens_from_payments(&self, payments: &ManagedVec<EsdtTokenPayment<Self::Api>>) {
        for entry in payments {
            self.nft_burn_tokens(&entry.token_identifier, entry.token_nonce, &entry.amount);
        }
    }

    fn get_merged_locked_asset_token_amount_and_attributes(
        &self,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> SCResult<(BigUint, LockedAssetTokenAttributes)> {
        require!(!payments.is_empty(), "Cannot merge with 0 tokens");

        let mut tokens = Vec::new();
        let mut sum_amount = BigUint::zero();
        let locked_asset_token_id = self.locked_asset_token_id().get();

        for entry in payments.iter() {
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
            return Ok((
                tokens[0].token_amount.amount.clone(),
                tokens[0].attributes.clone(),
            ));
        }

        let attrs = LockedAssetTokenAttributes {
            unlock_schedule: self.aggregated_unlock_schedule(&tokens)?,
            is_merged: true,
        };

        Ok((sum_amount, attrs))
    }

    fn aggregated_unlock_schedule(
        &self,
        tokens: &[LockedToken<Self::Api>],
    ) -> SCResult<UnlockSchedule> {
        let mut unlock_epoch_amount = Vec::new();
        tokens.iter().for_each(|locked_token| {
            locked_token
                .attributes
                .unlock_schedule
                .unlock_milestones
                .iter()
                .for_each(|milestone| {
                    unlock_epoch_amount.push((
                        milestone.unlock_epoch,
                        self.rule_of_three(
                            &self.types().big_uint_from(milestone.unlock_percent as u64),
                            &self.types().big_uint_from(PERCENTAGE_TOTAL as u64),
                            &locked_token.token_amount.amount,
                        ),
                    ))
                })
        });
        unlock_epoch_amount.sort_by(|a, b| a.0.cmp(&b.0));

        let mut sum = BigUint::zero();
        let default = (0u64, BigUint::zero());
        let mut unlock_epoch_amount_merged: Vec<(u64, BigUint)> = Vec::new();
        for elem in unlock_epoch_amount.iter() {
            let last = unlock_epoch_amount_merged.last().unwrap_or(&default);

            if elem.0 == last.0 {
                let new_elem = (last.0, &last.1 + &elem.1);
                unlock_epoch_amount_merged.pop();
                unlock_epoch_amount_merged.push(new_elem);
            } else {
                unlock_epoch_amount_merged.push(elem.clone());
            }

            sum += &elem.1;
        }
        require!(sum != 0, "Sum cannot be zero");
        require!(
            unlock_epoch_amount_merged.len() < MAX_MILESTONES_IN_SCHEDULE,
            "Too many milestones"
        );

        let mut new_unlock_milestones = Vec::new();
        unlock_epoch_amount_merged.iter().for_each(|x| {
            if x.1 != BigUint::zero() {
                let unlock_percent = &(&x.1 * 100u64) / &sum;

                if unlock_percent != 0 {
                    new_unlock_milestones.push(UnlockMilestone {
                        unlock_epoch: x.0,
                        unlock_percent: unlock_percent.to_u64().unwrap() as u8,
                    })
                }
            }
        });

        let mut sum_of_new_percents = 0u8;
        for new_milestone in new_unlock_milestones.iter() {
            sum_of_new_percents += new_milestone.unlock_percent;
        }
        new_unlock_milestones[0].unlock_percent += PERCENTAGE_TOTAL as u8 - sum_of_new_percents;

        Ok(UnlockSchedule {
            unlock_milestones: new_unlock_milestones,
        })
    }
}
