elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{GenericEsdtAmountPair, UnlockMilestone};

use super::locked_asset;
use super::locked_asset::{LockedAssetTokenAttributes, UnlockSchedule, PERCENTAGE_TOTAL};

const MAX_MILESTONES_IN_SCHEDULE: usize = 64;

pub struct LockedToken<BigUint: BigUintApi> {
    pub token_amount: GenericEsdtAmountPair<BigUint>,
    pub attributes: LockedAssetTokenAttributes,
}

#[elrond_wasm_derive::module]
pub trait TokenMergeModule:
    locked_asset::LockedAssetModule
    + token_supply::TokenSupplyModule
    + token_send::TokenSendModule
    + nft_deposit::NftDepositModule
{
    #[endpoint(mergeTokens)]
    fn merge_tokens(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let (amount, attrs) = self.get_merged_locked_asset_token_amount_and_attributes(&caller)?;
        let farm_token_id = self.locked_asset_token_id().get();

        self.burn_merge_tokens(&caller);
        self.nft_deposit(&caller).clear();

        self.nft_create_tokens(&farm_token_id, &amount, &attrs);
        self.increase_nonce();

        self.send_nft_tokens(
            &farm_token_id,
            self.locked_asset_token_nonce().get(),
            &amount,
            &caller,
            &opt_accept_funds_func,
        );

        Ok(())
    }

    fn get_merged_locked_asset_token_amount_and_attributes(
        &self,
        caller: &Address,
    ) -> SCResult<(Self::BigUint, LockedAssetTokenAttributes)> {
        let mut index = 1;
        let mut tokens = Vec::new();
        let deposit_len = self.nft_deposit(caller).len();
        require!(deposit_len != 0, "Cannot merge with 0 tokens");

        let locked_asset_token_id = self.locked_asset_token_id().get();
        let mut sum_amount = Self::BigUint::zero();

        while index <= deposit_len {
            let entry = self.nft_deposit(caller).get(index);
            require!(entry.token_id == locked_asset_token_id, "Bad token id");

            tokens.push(LockedToken {
                token_amount: entry.clone(),
                attributes: self.get_attributes(&entry.token_id, entry.token_nonce)?,
            });
            sum_amount += &entry.amount;
            index += 1;
        }

        if tokens.len() == 1 {
            return Ok((
                tokens[0].token_amount.amount.clone(),
                tokens[0].attributes.clone(),
            ));
        }

        let attrs = LockedAssetTokenAttributes {
            unlock_schedule: self.aggregated_unlock_schedule(&tokens)?,
        };

        Ok((sum_amount, attrs))
    }

    fn aggregated_unlock_schedule(
        &self,
        tokens: &[LockedToken<Self::BigUint>],
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
                            &Self::BigUint::from(milestone.unlock_percent as u64),
                            &Self::BigUint::from(PERCENTAGE_TOTAL as u64),
                            &locked_token.token_amount.amount,
                        ),
                    ))
                })
        });
        unlock_epoch_amount.sort_by(|a, b| a.0.cmp(&b.0));

        let mut sum = Self::BigUint::zero();
        let default = (0u64, Self::BigUint::zero());
        let mut unlock_epoch_amount_merged: Vec<(u64, Self::BigUint)> = Vec::new();
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
        require!(
            unlock_epoch_amount_merged.len() < MAX_MILESTONES_IN_SCHEDULE,
            "Too many milestones"
        );

        let mut new_unlock_milestones = Vec::new();
        unlock_epoch_amount_merged.iter().for_each(|x| {
            if x.1 != Self::BigUint::zero() {
                let unlock_percent = &(&x.1 * &Self::BigUint::from(100u64)) / &sum;
                new_unlock_milestones.push(UnlockMilestone {
                    unlock_epoch: x.0,
                    unlock_percent: unlock_percent.to_u64().unwrap() as u8,
                })
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

    fn burn_merge_tokens(&self, caller: &Address) {
        let deposit_len = self.nft_deposit(caller).len();
        let mut index = 1;

        while index <= deposit_len {
            let entry = self.nft_deposit(caller).get(index);
            self.nft_burn_tokens(&entry.token_id, entry.token_nonce, &entry.amount);
            index += 1;
        }
    }

    fn rule_of_three(
        &self,
        part: &Self::BigUint,
        total: &Self::BigUint,
        value: &Self::BigUint,
    ) -> Self::BigUint {
        &(part * value) / total
    }
}
