#![no_std]
#![feature(generic_associated_types)]
#![feature(exact_size_is_empty)]

mod attr_ex_helper;
mod cache;
mod events;
pub mod locked_asset;
pub mod locked_asset_token_merge;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const ADDITIONAL_AMOUNT_TO_CREATE: u64 = 1;
const EPOCHS_IN_MONTH: u64 = 30;

use attr_ex_helper::PRECISION_EX_INCREASE;
use common_structs::{
    Epoch, LockedAssetTokenAttributesEx, UnlockMilestone, UnlockMilestoneEx, UnlockPeriod,
    UnlockScheduleEx,
};

#[elrond_wasm::contract]
pub trait LockedAssetFactory:
    locked_asset::LockedAssetModule
    + cache::CacheModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
    + locked_asset_token_merge::LockedAssetTokenMergeModule
    + events::EventsModule
    + attr_ex_helper::AttrExHelper
{
    #[init]
    fn init(
        &self,
        asset_token_id: TokenIdentifier,
        default_unlock_period: MultiValueEncoded<UnlockMilestone>,
    ) {
        require!(
            asset_token_id.is_valid_esdt_identifier(),
            "Asset token ID is not a valid esdt identifier"
        );
        require!(
            asset_token_id != self.locked_asset_token().get_token_id(),
            "Asset token ID cannot be the same as Locked asset token ID"
        );
        let unlock_milestones = default_unlock_period.to_vec();
        self.validate_unlock_milestones(&unlock_milestones);

        let is_sc_upgrade = !self.init_epoch().is_empty();
        if is_sc_upgrade {
            self.set_extended_attributes_activation_nonce();
        } else {
            let current_epoch = self.blockchain().get_block_epoch();
            self.init_epoch().set(current_epoch);
        }

        self.asset_token_id().set(&asset_token_id);
        self.default_unlock_period()
            .set(&UnlockPeriod { unlock_milestones });
    }

    fn set_extended_attributes_activation_nonce(&self) {
        if self.extended_attributes_activation_nonce().is_empty() {
            let one = BigUint::from(1u64);
            let zero = BigUint::zero();
            let mb_empty = ManagedBuffer::new();
            let mv_empty = ManagedVec::new();
            let token_id = self.locked_asset_token().get_token_id();

            let nonce = self.send().esdt_nft_create(
                &token_id, &one, &mb_empty, &zero, &mb_empty, &mb_empty, &mv_empty,
            );
            self.send().esdt_local_burn(&token_id, nonce, &one);

            self.extended_attributes_activation_nonce()
                .set(&(nonce + 1));
        }
    }

    #[only_owner]
    #[endpoint]
    fn whitelist(&self, address: ManagedAddress) {
        let _ = self.whitelisted_contracts().insert(address);
    }

    #[only_owner]
    #[endpoint(removeWhitelist)]
    fn remove_whitelist(&self, address: ManagedAddress) {
        let _ = self.whitelisted_contracts().remove(&address);
    }

    #[endpoint(createAndForwardCustomPeriod)]
    fn create_and_forward_custom_period(
        &self,
        amount: BigUint,
        address: ManagedAddress,
        start_epoch: Epoch,
        unlock_period: UnlockPeriod<Self::Api>,
    ) -> EsdtTokenPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        require!(
            self.whitelisted_contracts().contains(&caller),
            "Permission denied"
        );
        require!(!unlock_period.unlock_milestones.is_empty(), "Empty arg");

        let month_start_epoch = self.get_month_start_epoch(start_epoch);
        let attr = LockedAssetTokenAttributesEx {
            unlock_schedule: self.create_unlock_schedule(month_start_epoch, unlock_period),
            is_merged: false,
        };

        let new_token = self.produce_tokens_and_send(&amount, &attr, &address);

        self.emit_create_and_forward_event(
            &caller,
            &address,
            &new_token.token_identifier,
            new_token.token_nonce,
            &new_token.amount,
            &attr,
            month_start_epoch,
        );
        new_token
    }

    #[endpoint(createAndForward)]
    fn create_and_forward(
        &self,
        amount: BigUint,
        address: ManagedAddress,
        start_epoch: Epoch,
    ) -> EsdtTokenPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        require!(
            self.whitelisted_contracts().contains(&caller),
            "Permission denied"
        );
        require!(
            !self.locked_asset_token().is_empty(),
            "Locked Asset Token not registered"
        );
        require!(amount > 0, "Zero input amount");

        self.common_create_and_forward(amount, address, caller, start_epoch)
    }

    #[payable("*")]
    #[endpoint(unlockAssets)]
    fn unlock_assets(&self) {
        let (token_id, token_nonce, amount) = self.call_value().single_esdt().into_tuple();
        let locked_token_id = self.locked_asset_token().get_token_id();
        require!(token_id == locked_token_id, "Bad payment token");

        let attributes = self.get_attributes_ex(&token_id, token_nonce);
        let unlock_schedule = &attributes.unlock_schedule;

        let month_start_epoch = self.get_month_start_epoch(self.blockchain().get_block_epoch());
        let unlock_amount = self.get_unlock_amount(
            &amount,
            month_start_epoch,
            &unlock_schedule.unlock_milestones,
        );
        require!(amount >= unlock_amount, "Cannot unlock more than locked");
        require!(unlock_amount > 0u64, "Method called too soon");

        let caller = self.blockchain().get_caller();
        self.mint_and_send_assets(&caller, &unlock_amount);

        let mut output_locked_assets_token_amount =
            EsdtTokenPayment::new(token_id.clone(), 0, BigUint::zero());
        let mut output_locked_asset_attributes = LockedAssetTokenAttributesEx {
            unlock_schedule: UnlockScheduleEx {
                unlock_milestones: ManagedVec::new(),
            },
            is_merged: false,
        };

        let locked_remaining = &amount - &unlock_amount;
        if locked_remaining > 0u64 {
            let new_unlock_milestones = self.create_new_unlock_milestones(
                month_start_epoch,
                &unlock_schedule.unlock_milestones,
            );
            output_locked_asset_attributes = LockedAssetTokenAttributesEx {
                unlock_schedule: UnlockScheduleEx {
                    unlock_milestones: new_unlock_milestones,
                },
                is_merged: attributes.is_merged,
            };
            output_locked_assets_token_amount = self.produce_tokens_and_send(
                &locked_remaining,
                &output_locked_asset_attributes,
                &caller,
            );
        }

        self.send()
            .esdt_local_burn(&locked_token_id, token_nonce, &amount);

        self.emit_unlock_assets_event(
            &caller,
            &token_id,
            token_nonce,
            &amount,
            &output_locked_assets_token_amount.token_identifier,
            output_locked_assets_token_amount.token_nonce,
            &output_locked_assets_token_amount.amount,
            &self.asset_token_id().get(),
            &unlock_amount,
            &attributes,
            &output_locked_asset_attributes,
        );
    }

    #[payable("*")]
    #[endpoint(lockAssets)]
    fn lock_assets(&self) -> EsdtTokenPayment<Self::Api> {
        let (payment_token, payment_amount) = self.call_value().single_fungible_esdt();
        let caller = self.blockchain().get_caller();

        let asset_token_id = self.asset_token_id().get();
        require!(payment_token == asset_token_id, "INVALID TOKEN PAYMENT");
        let block_epoch = self.blockchain().get_block_epoch();

        self.send()
            .esdt_local_burn(&payment_token, 0, &payment_amount);

        self.common_create_and_forward(payment_amount, caller.clone(), caller, block_epoch)
    }

    #[only_owner]
    #[endpoint(setUnlockPeriod)]
    fn set_unlock_period(&self, milestones: MultiValueEncoded<UnlockMilestone>) {
        let unlock_milestones = milestones.to_vec();
        self.validate_unlock_milestones(&unlock_milestones);
        self.default_unlock_period()
            .set(&UnlockPeriod { unlock_milestones });
    }

    fn get_month_start_epoch(&self, epoch: Epoch) -> Epoch {
        epoch - (epoch - self.init_epoch().get()) % EPOCHS_IN_MONTH
    }

    fn common_create_and_forward(
        &self,
        amount: BigUint,
        address: ManagedAddress,
        caller: ManagedAddress,
        start_epoch: Epoch,
    ) -> EsdtTokenPayment<Self::Api> {
        let month_start_epoch = self.get_month_start_epoch(start_epoch);
        let unlock_period = self.default_unlock_period().get();
        let attr = LockedAssetTokenAttributesEx {
            unlock_schedule: self.create_unlock_schedule(month_start_epoch, unlock_period),
            is_merged: false,
        };

        let new_token = self.produce_tokens_and_send(&amount, &attr, &address);

        self.emit_create_and_forward_event(
            &caller,
            &address,
            &new_token.token_identifier,
            new_token.token_nonce,
            &new_token.amount,
            &attr,
            start_epoch,
        );
        new_token
    }

    fn produce_tokens_and_send(
        &self,
        amount: &BigUint,
        attributes: &LockedAssetTokenAttributesEx<Self::Api>,
        address: &ManagedAddress,
    ) -> EsdtTokenPayment<Self::Api> {
        let result = self.get_sft_nonce_for_unlock_schedule(&attributes.unlock_schedule);
        match result {
            Option::Some(cached_nonce) => {
                self.add_quantity_and_send_locked_assets(amount.clone(), cached_nonce, address)
            }
            Option::None => {
                let do_cache_result = !attributes.is_merged;

                let additional_amount_to_create = if do_cache_result {
                    BigUint::from(ADDITIONAL_AMOUNT_TO_CREATE)
                } else {
                    BigUint::zero()
                };

                let new_tokens = self.create_and_send_locked_assets(
                    amount,
                    &additional_amount_to_create,
                    address,
                    attributes,
                );

                if do_cache_result {
                    self.cache_unlock_schedule_and_nonce(
                        &attributes.unlock_schedule,
                        new_tokens.token_nonce,
                    );
                }

                new_tokens
            }
        }
    }

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerLockedAssetToken)]
    fn register_locked_asset_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        let payment_amount = self.call_value().egld_value();
        self.locked_asset_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            payment_amount,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    fn create_unlock_schedule(
        &self,
        start_epoch: Epoch,
        unlock_period: UnlockPeriod<Self::Api>,
    ) -> UnlockScheduleEx<Self::Api> {
        let mut result = ManagedVec::new();
        for milestone in unlock_period.unlock_milestones.iter() {
            result.push(UnlockMilestoneEx {
                unlock_epoch: milestone.unlock_epoch + start_epoch,
                unlock_percent: milestone.unlock_percent as u64 * PRECISION_EX_INCREASE,
            });
        }
        UnlockScheduleEx {
            unlock_milestones: result,
        }
    }

    #[only_owner]
    #[endpoint(setInitEpoch)]
    fn set_init_epoch(&self, init_epoch: Epoch) {
        self.init_epoch().set(&init_epoch);
    }

    #[view(getInitEpoch)]
    #[storage_mapper("init_epoch")]
    fn init_epoch(&self) -> SingleValueMapper<Epoch>;

    #[view(getWhitelistedContracts)]
    #[storage_mapper("whitelist")]
    fn whitelisted_contracts(&self) -> SetMapper<ManagedAddress>;

    #[view(getDefaultUnlockPeriod)]
    #[storage_mapper("default_unlock_period")]
    fn default_unlock_period(&self) -> SingleValueMapper<UnlockPeriod<Self::Api>>;
}
