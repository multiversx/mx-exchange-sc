#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const DEFAULT_TRANSFER_EXEC_GAS_LIMIT: u64 = 25000000;

use dex_common::*;
use distrib_common::*;
use modules::*;

mod cache;
mod locked_asset;

#[elrond_wasm_derive::contract]
pub trait LockedAssetFactory:
    asset::AssetModule + locked_asset::LockedAssetModule + cache::CacheModule
{
    #[init]
    fn init(
        &self,
        asset_token_id: TokenIdentifier,
        #[var_args] default_unlock_period: VarArgs<UnlockMilestone>,
    ) -> SCResult<()> {
        require!(!default_unlock_period.is_empty(), "Empty param");
        self.validate_unlock_milestones(&default_unlock_period)?;

        self.asset_token_id().set(&asset_token_id);
        self.default_unlock_period().set(&default_unlock_period.0);
        self.transfer_exec_gas_limit()
            .set(&DEFAULT_TRANSFER_EXEC_GAS_LIMIT);
        Ok(())
    }

    #[endpoint]
    fn whitelist(&self, address: Address) -> SCResult<()> {
        only_owner!(self, "Permission denied");

        self.whitelisted_contracts().insert(address);
        Ok(())
    }

    #[endpoint(removeWhitelist)]
    fn remove_whitelist(&self, address: Address) -> SCResult<()> {
        only_owner!(self, "Permission denied");

        self.whitelisted_contracts().remove(&address);
        Ok(())
    }

    #[endpoint(createAndForward)]
    fn create_and_forward(
        &self,
        amount: Self::BigUint,
        address: Address,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<GenericEsdtAmountPair<Self::BigUint>> {
        let caller = self.blockchain().get_caller();
        require!(
            self.whitelisted_contracts().contains(&caller),
            "Permission denied"
        );
        require!(!self.locked_asset_token_id().is_empty(), "No SFT issued");
        require!(amount > 0, "Zero input amount");

        self.produce_tokens_and_send(
            &amount,
            &self.create_default_unlock_milestones(),
            &address,
            &opt_accept_funds_func,
        )
    }

    #[endpoint(createAndForwardCustomSchedule)]
    fn create_and_forward_custom_schedule(
        &self,
        amount: Self::BigUint,
        address: Address,
        #[var_args] schedule: VarArgs<UnlockMilestone>,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        require!(
            self.whitelisted_contracts().contains(&caller),
            "Permission denied"
        );
        require!(!self.locked_asset_token_id().is_empty(), "No SFT issued");
        require!(amount > 0, "Zero input amount");
        require!(!schedule.is_empty(), "Empty param");

        let _ = self.produce_tokens_and_send(&amount, &schedule.0, &address, &OptionalArg::None);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(unlockAssets)]
    fn unlock_assets(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment] amount: Self::BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let token_nonce = self.call_value().esdt_token_nonce();
        let locked_token_id = self.locked_asset_token_id().get();
        require!(token_id == locked_token_id, "Bad payment token");

        let attributes = self.get_attributes(&token_id, token_nonce)?;
        let current_block_epoch = self.blockchain().get_block_epoch();
        let unlock_amount =
            self.get_unlock_amount(&amount, current_block_epoch, &attributes.unlock_milestones);
        require!(amount >= unlock_amount, "Cannot unlock more than locked");
        require!(unlock_amount > 0, "Method called too soon");

        let caller = self.blockchain().get_caller();
        self.mint_and_send_assets(&caller, &unlock_amount);

        let locked_remaining = amount.clone() - unlock_amount;
        if locked_remaining > 0 {
            let new_unlock_milestones = self
                .create_new_unlock_milestones(current_block_epoch, &attributes.unlock_milestones);
            let _ = self.produce_tokens_and_send(
                &locked_remaining,
                &new_unlock_milestones,
                &caller,
                &opt_accept_funds_func,
            );
        }

        self.burn_locked_assets(&locked_token_id, &amount, token_nonce);
        Ok(())
    }

    fn produce_tokens_and_send(
        &self,
        amount: &Self::BigUint,
        unlock_milestones: &[UnlockMilestone],
        address: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) -> SCResult<GenericEsdtAmountPair<Self::BigUint>> {
        let attributes = LockedTokenAttributes {
            unlock_milestones: unlock_milestones.to_vec(),
        };
        let result = self.get_cached_sft_nonce_for_attributes(&attributes);
        let sent_nonce = match result {
            Option::Some(cached_nonce) => {
                self.add_quantity_and_send_locked_assets(
                    &amount,
                    cached_nonce,
                    &address,
                    opt_accept_funds_func,
                )?;
                cached_nonce
            }
            Option::None => {
                let new_nonce = self.create_and_send_locked_assets(
                    &amount,
                    &attributes,
                    &address,
                    opt_accept_funds_func,
                )?;
                self.cache_attributes_and_nonce(attributes, new_nonce);
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
    #[endpoint(issueNft)]
    fn issue_nft(
        &self,
        token_display_name: BoxedBytes,
        token_ticker: BoxedBytes,
        #[payment] issue_cost: Self::BigUint,
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
                self.locked_asset_token_id().set(&token_id);
            }
            AsyncCallResult::Err(_) => {
                // return payment to initial caller, which can only be the owner
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

    #[endpoint(setLocalRoles)]
    fn set_local_roles(
        &self,
        token: TokenIdentifier,
        address: Address,
        #[var_args] roles: VarArgs<EsdtLocalRole>,
    ) -> SCResult<AsyncCall<Self::SendApi>> {
        only_owner!(self, "Permission denied");
        require!(token == self.locked_asset_token_id().get(), "Bad token id");
        require!(!roles.is_empty(), "Empty roles");

        Ok(ESDTSystemSmartContractProxy::new_proxy_obj(self.send())
            .set_special_roles(&address, token.as_esdt_identifier(), &roles.as_slice())
            .async_call())
    }

    fn create_default_unlock_milestones(&self) -> Vec<UnlockMilestone> {
        let current_epoch = self.blockchain().get_block_epoch();

        self.default_unlock_period()
            .get()
            .iter()
            .map(|x| UnlockMilestone {
                unlock_epoch: x.unlock_epoch + current_epoch,
                unlock_percent: x.unlock_percent,
            })
            .collect()
    }

    #[storage_mapper("whitelist")]
    fn whitelisted_contracts(&self) -> SetMapper<Self::Storage, Address>;

    #[storage_mapper("default_unlock_period")]
    fn default_unlock_period(&self) -> SingleValueMapper<Self::Storage, Vec<UnlockMilestone>>;
}
