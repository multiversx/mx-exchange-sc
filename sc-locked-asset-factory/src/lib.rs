#![no_std]

mod cache;
mod locked_asset;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const DEFAULT_TRANSFER_EXEC_GAS_LIMIT: u64 = 35000000;
const EPOCHS_IN_MONTH: u64 = 30;

type Epoch = u64;
type Nonce = u64;

use dex_common::GenericEsdtAmountPair;
use distrib_common::UnlockMilestone;
use locked_asset::UnlockSchedule;

#[elrond_wasm_derive::contract]
pub trait LockedAssetFactory:
    locked_asset::LockedAssetModule + cache::CacheModule + token_supply::TokenSupplyModule
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
        self.produce_tokens_and_send(
            &amount,
            &self.create_default_unlock_schedule(month_start_epoch),
            &address,
            &opt_accept_funds_func,
        )
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

        let cached_value = self.get_unlock_schedule_for_sft_nonce(token_nonce);
        require!(cached_value.is_some(), "Unlock Schedule not found in cache");
        let unlock_schedule = cached_value.unwrap();

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
            let new_unlock_schedule = UnlockSchedule {
                unlock_milestones: new_unlock_milestones,
            };
            let _ = self.produce_tokens_and_send(
                &locked_remaining,
                &new_unlock_schedule,
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
        unlock_schedule: &UnlockSchedule,
        address: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) -> SCResult<GenericEsdtAmountPair<Self::BigUint>> {
        let result = self.get_sft_nonce_for_unlock_schedule(unlock_schedule);
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
                let new_nonce =
                    self.create_and_send_locked_assets(amount, address, opt_accept_funds_func);
                self.cache_unlock_schedule_and_nonce(unlock_schedule, new_nonce);
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
                self.locked_asset_token_id().set(&token_id);
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
