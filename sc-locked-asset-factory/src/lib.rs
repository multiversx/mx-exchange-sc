#![no_std]
#![allow(non_snake_case)]

mod cache;
mod locked_asset;
mod locked_asset_token_merge;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const DEFAULT_TRANSFER_EXEC_GAS_LIMIT: u64 = 35000000;
const DEFAULT_NFT_DEPOSIT_MAX_LEN: usize = 10;
const ADDITIONAL_AMOUNT_TO_CREATE: u64 = 1;
const EPOCHS_IN_MONTH: u64 = 30;

use common_structs::{Epoch, GenericEsdtAmountPair, Nonce, UnlockMilestone};
use locked_asset::UnlockSchedule;

use crate::locked_asset::LockedAssetTokenAttributes;

#[elrond_wasm_derive::contract]
pub trait LockedAssetFactory:
    locked_asset::LockedAssetModule
    + cache::CacheModule
    + token_supply::TokenSupplyModule
    + token_send::TokenSendModule
    + nft_deposit::NftDepositModule
    + token_merge::TokenMergeModule
    + locked_asset_token_merge::LockedAssetTokenMergeModule
{
    #[init]
    fn init(
        &self,
        asset_token_id: TokenIdentifier,
        #[var_args] default_unlock_period: VarArgs<UnlockMilestone>,
    ) -> SCResult<()> {
        require!(
            asset_token_id.is_valid_esdt_identifier(),
            "Asset token ID is not a valid esdt identifier"
        );
        require!(
            asset_token_id != self.locked_asset_token_id().get(),
            "Asset token ID cannot be the same as Locked asset token ID"
        );
        self.validate_unlock_milestones(&default_unlock_period)?;

        self.transfer_exec_gas_limit()
            .set_if_empty(&DEFAULT_TRANSFER_EXEC_GAS_LIMIT);
        self.init_epoch()
            .set_if_empty(&self.blockchain().get_block_epoch());
        self.nft_deposit_max_len()
            .set_if_empty(&DEFAULT_NFT_DEPOSIT_MAX_LEN);

        self.asset_token_id().set(&asset_token_id);
        self.default_unlock_period().set(&default_unlock_period.0);
        Ok(())
    }

    #[endpoint]
    fn whitelist(&self, address: Address) -> SCResult<()> {
        only_owner!(self, "Permission denied");

        let is_new = self.whitelisted_contracts().insert(address);
        require!(is_new, "Address already whitelisted");
        Ok(())
    }

    #[endpoint(removeWhitelist)]
    fn remove_whitelist(&self, address: Address) -> SCResult<()> {
        only_owner!(self, "Permission denied");

        let is_removed = self.whitelisted_contracts().remove(&address);
        require!(is_removed, "Addresss not whitelisted");
        Ok(())
    }

    #[endpoint(createAndForward)]
    fn create_and_forward(
        &self,
        amount: Self::BigUint,
        address: Address,
        start_epoch: Epoch,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<GenericEsdtAmountPair<Self::BigUint>> {
        let caller = self.blockchain().get_caller();
        require!(
            self.whitelisted_contracts().contains(&caller),
            "Permission denied"
        );
        require!(!self.locked_asset_token_id().is_empty(), "No SFT issued");
        require!(amount > 0, "Zero input amount");
        require!(
            start_epoch >= self.init_epoch().get(),
            "Invalid start epoch"
        );

        let month_start_epoch = self.get_month_start_epoch(start_epoch);
        let attr = LockedAssetTokenAttributes {
            unlock_schedule: self.create_default_unlock_schedule(month_start_epoch),
            is_merged: false,
        };

        self.produce_tokens_and_send(&amount, &attr, &address, &opt_accept_funds_func)
    }

    #[payable("*")]
    #[endpoint(unlockAssets)]
    fn unlock_assets(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
        #[payment_nonce] token_nonce: Nonce,
    ) -> SCResult<()> {
        let locked_token_id = self.locked_asset_token_id().get();
        require!(token_id == locked_token_id, "Bad payment token");

        let attributes = self.get_attributes(&token_id, token_nonce)?;
        let unlock_schedule = attributes.unlock_schedule;

        let month_start_epoch = self.get_month_start_epoch(self.blockchain().get_block_epoch());
        let unlock_amount = self.get_unlock_amount(
            &amount,
            month_start_epoch,
            &unlock_schedule.unlock_milestones,
        );
        require!(amount >= unlock_amount, "Cannot unlock more than locked");
        require!(unlock_amount > 0, "Method called too soon");

        let caller = self.blockchain().get_caller();
        self.mint_and_send_assets(&caller, &unlock_amount);

        let locked_remaining = amount.clone() - unlock_amount;
        if locked_remaining > 0 {
            let new_unlock_milestones = self.create_new_unlock_milestones(
                month_start_epoch,
                &unlock_schedule.unlock_milestones,
            );
            let new_attributes = LockedAssetTokenAttributes {
                unlock_schedule: UnlockSchedule {
                    unlock_milestones: new_unlock_milestones,
                },
                is_merged: attributes.is_merged,
            };
            let _ = self.produce_tokens_and_send(
                &locked_remaining,
                &new_attributes,
                &caller,
                &OptionalArg::None,
            );
        }

        self.nft_burn_tokens(&locked_token_id, token_nonce, &amount);
        Ok(())
    }

    fn get_month_start_epoch(&self, epoch: Epoch) -> Epoch {
        epoch - (epoch - self.init_epoch().get()) % EPOCHS_IN_MONTH
    }

    fn produce_tokens_and_send(
        &self,
        amount: &Self::BigUint,
        attributes: &LockedAssetTokenAttributes,
        address: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) -> SCResult<GenericEsdtAmountPair<Self::BigUint>> {
        let result = self.get_sft_nonce_for_unlock_schedule(&attributes.unlock_schedule);
        let sent_nonce = match result {
            Option::Some(cached_nonce) => {
                self.add_quantity_and_send_locked_assets(
                    amount,
                    cached_nonce,
                    address,
                    opt_accept_funds_func,
                );
                cached_nonce
            }
            Option::None => {
                let do_cache_result = !attributes.is_merged;

                let additional_amount_to_create = if do_cache_result {
                    Self::BigUint::from(ADDITIONAL_AMOUNT_TO_CREATE)
                } else {
                    Self::BigUint::zero()
                };

                let new_nonce = self.create_and_send_locked_assets(
                    amount,
                    &additional_amount_to_create,
                    address,
                    attributes,
                    opt_accept_funds_func,
                );

                if do_cache_result {
                    self.cache_unlock_schedule_and_nonce(&attributes.unlock_schedule, new_nonce);
                }
                new_nonce
            }
        };
        Ok(GenericEsdtAmountPair {
            token_id: self.locked_asset_token_id().get(),
            token_nonce: sent_nonce,
            amount: amount.clone(),
        })
    }

    #[payable("EGLD")]
    #[endpoint(issueLockedAssetToken)]
    fn issue_locked_asset_token(
        &self,
        token_display_name: BoxedBytes,
        token_ticker: BoxedBytes,
        #[payment_amount] issue_cost: Self::BigUint,
    ) -> SCResult<AsyncCall<Self::SendApi>> {
        only_owner!(self, "Permission denied");
        require!(
            self.locked_asset_token_id().is_empty(),
            "NFT already issued"
        );

        Ok(ESDTSystemSmartContractProxy::new_proxy_obj(self.send())
            .issue_semi_fungible(
                issue_cost,
                &token_display_name,
                &token_ticker,
                SemiFungibleTokenProperties {
                    can_add_special_roles: true,
                    can_change_owner: false,
                    can_freeze: false,
                    can_pause: false,
                    can_upgrade: true,
                    can_wipe: false,
                },
            )
            .async_call()
            .with_callback(self.callbacks().issue_nft_callback()))
    }

    #[callback]
    fn issue_nft_callback(&self, #[call_result] result: AsyncCallResult<TokenIdentifier>) {
        match result {
            AsyncCallResult::Ok(token_id) => {
                self.last_error_message().clear();

                if self.locked_asset_token_id().is_empty() {
                    self.locked_asset_token_id().set(&token_id);
                    self.nft_deposit_accepted_token_ids().insert(token_id);
                }
            }
            AsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);

                let (payment, token_id) = self.call_value().payment_token_pair();
                self.send().direct(
                    &self.blockchain().get_owner_address(),
                    &token_id,
                    &payment,
                    &[],
                );
            }
        };
    }

    #[endpoint(setLocalRolesLockedAssetToken)]
    fn set_local_roles_locked_asset_token(
        &self,
        address: Address,
        #[var_args] roles: VarArgs<EsdtLocalRole>,
    ) -> SCResult<AsyncCall<Self::SendApi>> {
        only_owner!(self, "Permission denied");
        require!(
            !self.locked_asset_token_id().is_empty(),
            "Locked asset SFT not issued"
        );
        require!(!roles.is_empty(), "Empty roles");

        let token = self.locked_asset_token_id().get();
        Ok(ESDTSystemSmartContractProxy::new_proxy_obj(self.send())
            .set_special_roles(&address, &token, roles.as_slice())
            .async_call()
            .with_callback(self.callbacks().change_roles_callback()))
    }

    #[callback]
    fn change_roles_callback(&self, #[call_result] result: AsyncCallResult<()>) {
        match result {
            AsyncCallResult::Ok(()) => {
                self.last_error_message().clear();
            }
            AsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);
            }
        }
    }

    #[payable("*")]
    #[endpoint]
    fn depositLockedAssetTokens(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_nonce] payment_token_nonce: Nonce,
        #[payment_amount] payment_amount: Self::BigUint,
    ) -> SCResult<()> {
        self.deposit_tokens(payment_token_id, payment_token_nonce, payment_amount)
    }

    #[endpoint]
    fn mergeLockedAssetTokens(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<GenericEsdtAmountPair<Self::BigUint>> {
        self.merge_and_send_tokens(opt_accept_funds_func)
    }

    #[endpoint(setNftDepositMaxLen)]
    fn set_nft_deposit_max_len(&self, max_len: usize) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.nft_deposit_max_len().set(&max_len);
        Ok(())
    }

    fn create_default_unlock_schedule(&self, start_epoch: Epoch) -> UnlockSchedule {
        UnlockSchedule {
            unlock_milestones: self
                .default_unlock_period()
                .get()
                .iter()
                .map(|x| UnlockMilestone {
                    unlock_epoch: x.unlock_epoch + start_epoch,
                    unlock_percent: x.unlock_percent,
                })
                .collect(),
        }
    }

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<Self::Storage, BoxedBytes>;

    #[view(getInitEpoch)]
    #[storage_mapper("init_epoch")]
    fn init_epoch(&self) -> SingleValueMapper<Self::Storage, Epoch>;

    #[view(getWhitelistedContracts)]
    #[storage_mapper("whitelist")]
    fn whitelisted_contracts(&self) -> SetMapper<Self::Storage, Address>;

    #[view(getDefaultUnlockPeriod)]
    #[storage_mapper("default_unlock_period")]
    fn default_unlock_period(&self) -> SingleValueMapper<Self::Storage, Vec<UnlockMilestone>>;
}
