elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{
    GenericTokenAmountPair, LockedAssetTokenAttributes, UnlockMilestone, UnlockSchedule,
};

use super::locked_asset;
use super::locked_asset::PERCENTAGE_TOTAL;

const MAX_MILESTONES_IN_SCHEDULE: usize = 64;

pub struct LockedToken<BigUint: BigUintApi> {
    pub token_amount: GenericTokenAmountPair<BigUint>,
    pub attributes: LockedAssetTokenAttributes,
}

#[elrond_wasm::module]
pub trait LockedAssetTokenMergeModule:
    locked_asset::LockedAssetModule
    + token_supply::TokenSupplyModule
    + token_send::TokenSendModule
    + nft_deposit::NftDepositModule
    + token_merge::TokenMergeModule
{
    #[endpoint(mergeLockedAssetTokens)]
    fn merge_locked_asset_tokens(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<GenericTokenAmountPair<Self::BigUint>> {
        let caller = self.blockchain().get_caller();
        let deposit = self.nft_deposit(&caller).get();
        require!(!deposit.is_empty(), "Empty deposit");

        let (amount, attrs) = self.get_merged_locked_asset_token_amount_and_attributes(&deposit)?;
        let locked_asset_token = self.locked_asset_token_id().get();

        self.burn_deposit_tokens(&caller, &deposit);

        self.nft_create_tokens(&locked_asset_token, &amount, &attrs);
        self.increase_nonce();

        let new_nonce = self.locked_asset_token_nonce().get();
        self.send_nft_tokens(
            &locked_asset_token,
            new_nonce,
            &amount,
            &caller,
            &opt_accept_funds_func,
        )?;

        Ok(GenericTokenAmountPair {
            token_id: locked_asset_token,
            token_nonce: new_nonce,
            amount,
        })
    }

    fn get_merged_locked_asset_token_amount_and_attributes(
        &self,
        deposit: &[GenericTokenAmountPair<Self::BigUint>],
    ) -> SCResult<(Self::BigUint, LockedAssetTokenAttributes)> {
        require!(!deposit.is_empty(), "Cannot merge with 0 tokens");

        let mut tokens = Vec::new();
        let mut sum_amount = 0u64.into();
        let locked_asset_token_id = self.locked_asset_token_id().get();

        for entry in deposit.iter() {
            require!(entry.token_id == locked_asset_token_id, "Bad token id");

            tokens.push(LockedToken {
                token_amount: entry.clone(),
                attributes: self.get_attributes(&entry.token_id, entry.token_nonce)?,
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
                            &(milestone.unlock_percent as u64).into(),
                            &(PERCENTAGE_TOTAL as u64).into(),
                            &locked_token.token_amount.amount,
                        ),
                    ))
                })
        });
        unlock_epoch_amount.sort_by(|a, b| a.0.cmp(&b.0));

        let mut sum = 0u64.into();
        let default = (0u64, 0u64.into());
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
        require!(sum != 0, "Sum cannot be zero");
        require!(
            unlock_epoch_amount_merged.len() < MAX_MILESTONES_IN_SCHEDULE,
            "Too many milestones"
        );

        let mut new_unlock_milestones = Vec::new();
        unlock_epoch_amount_merged.iter().for_each(|x| {
            if x.1 != Self::BigUint::zero() {
                let unlock_percent = &(&x.1 * &100u64.into()) / &sum;

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
