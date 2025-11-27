#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use attr_ex_helper::PRECISION_EX_INCREASE;
use common_structs::{
    Epoch, LockedAssetTokenAttributesEx, UnlockMilestoneEx, UnlockPeriod, UnlockScheduleEx,
};

pub mod attr_ex_helper;
mod cache;
mod events;
pub mod locked_asset;
pub mod locked_asset_token_merge;
pub mod migration;

const ADDITIONAL_AMOUNT_TO_CREATE: u64 = 1;
const EPOCHS_IN_MONTH: u64 = 30;

#[type_abi]
#[derive(TopEncode, TopDecode)]
pub struct OldEsdtTokenPayment<M: ManagedTypeApi> {
    pub token_type: EsdtTokenType,
    pub payment: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> OldEsdtTokenPayment<M> {
    pub fn new(payment: EsdtTokenPayment<M>) -> Self {
        Self {
            token_type: payment.token_type(),
            payment,
        }
    }
}

#[multiversx_sc::contract]
pub trait LockedAssetFactory:
    locked_asset::LockedAssetModule
    + cache::CacheModule
    + token_merge_helper::TokenMergeHelperModule
    + locked_asset_token_merge::LockedAssetTokenMergeModule
    + events::EventsModule
    + attr_ex_helper::AttrExHelper
    + migration::LockedTokenMigrationModule
    + multiversx_sc_modules::pause::PauseModule
{
    #[init]
    fn init(&self) {
        self.set_paused(true);
    }

    #[upgrade]
    fn upgrade(&self) {}

    #[only_owner]
    #[endpoint]
    fn whitelist(&self, address: ManagedAddress) {
        let is_new = self.whitelisted_contracts().insert(address);
        require!(is_new, "ManagedAddress already whitelisted");
    }

    #[only_owner]
    #[endpoint(removeWhitelist)]
    fn remove_whitelist(&self, address: ManagedAddress) {
        let is_removed = self.whitelisted_contracts().remove(&address);
        require!(is_removed, "ManagedAddresss not whitelisted");
    }

    #[endpoint(createAndForwardCustomPeriod)]
    fn create_and_forward_custom_period(
        &self,
        amount: BigUint,
        address: ManagedAddress,
        start_epoch: Epoch,
        unlock_period: UnlockPeriod<Self::Api>,
    ) -> OldEsdtTokenPayment<Self::Api> {
        self.require_not_paused();

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

        OldEsdtTokenPayment::new(new_token)
    }

    #[endpoint(createAndForward)]
    fn create_and_forward(
        &self,
        amount: BigUint,
        address: ManagedAddress,
        start_epoch: Epoch,
    ) -> OldEsdtTokenPayment<Self::Api> {
        self.require_not_paused();

        let caller = self.blockchain().get_caller();
        require!(
            self.whitelisted_contracts().contains(&caller),
            "Permission denied"
        );
        require!(
            !self.locked_asset_token_id().is_empty(),
            "Locked Asset Token not registered"
        );
        require!(amount > 0, "Zero input amount");

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

        OldEsdtTokenPayment::new(new_token)
    }

    #[payable("*")]
    #[endpoint(unlockAssets)]
    fn unlock_assets(&self) {
        self.require_not_paused();

        let (token_id, token_nonce, amount) = self.call_value().single_esdt().clone().into_tuple();
        let locked_token_id = self.locked_asset_token_id().get();
        require!(token_id == locked_token_id, "Bad payment token");

        let attr_ex_activation = self.extended_attributes_activation_nonce().get();
        let attributes = self.get_attributes_ex(&token_id, token_nonce, attr_ex_activation);
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

        let initial_amounts_per_epoch = attributes.get_unlock_amounts_per_epoch(&amount);
        let final_amounts_per_epoch =
            output_locked_asset_attributes.get_unlock_amounts_per_epoch(&locked_remaining);
        self.update_energy_after_unlock(
            caller.clone(),
            initial_amounts_per_epoch,
            final_amounts_per_epoch,
        );

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

    fn get_month_start_epoch(&self, epoch: Epoch) -> Epoch {
        epoch - (epoch - self.init_epoch().get()) % EPOCHS_IN_MONTH
    }

    fn produce_tokens_and_send(
        &self,
        amount: &BigUint,
        attributes: &LockedAssetTokenAttributesEx<Self::Api>,
        address: &ManagedAddress,
    ) -> EsdtTokenPayment<Self::Api> {
        let result = self.get_sft_nonce_for_unlock_schedule(&attributes.unlock_schedule);
        let sent_nonce = match result {
            Option::Some(cached_nonce) => {
                self.add_quantity_and_send_locked_assets(amount, cached_nonce, address);
                cached_nonce
            }
            Option::None => {
                let do_cache_result = !attributes.is_merged;

                let additional_amount_to_create = if do_cache_result {
                    BigUint::from(ADDITIONAL_AMOUNT_TO_CREATE)
                } else {
                    BigUint::zero()
                };

                let new_nonce = self.create_and_send_locked_assets(
                    amount,
                    &additional_amount_to_create,
                    address,
                    attributes,
                );

                if do_cache_result {
                    self.cache_unlock_schedule_and_nonce(&attributes.unlock_schedule, new_nonce);
                }
                new_nonce
            }
        };

        let token_id = self.locked_asset_token_id().get();
        EsdtTokenPayment::new(token_id, sent_nonce, amount.clone())
    }

    #[only_owner]
    #[endpoint(setTransferRoleForAddress)]
    fn set_transfer_role_for_address(&self, opt_address: OptionalValue<ManagedAddress>) {
        let address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &address,
                &self.locked_asset_token_id().get(),
                [EsdtLocalRole::Transfer][..].iter().cloned(),
            )
            .async_call_and_exit()
    }

    #[only_owner]
    #[endpoint(unsetTransferRoleForAddress)]
    fn unset_transfer_role_for_address(&self, opt_address: OptionalValue<ManagedAddress>) {
        let address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };

        self.send()
            .esdt_system_sc_proxy()
            .unset_special_roles(
                &address,
                &self.locked_asset_token_id().get(),
                [EsdtLocalRole::Transfer][..].iter().cloned(),
            )
            .async_call_and_exit()
    }

    #[only_owner]
    #[endpoint(setBurnRoleForAddress)]
    fn set_burn_role_for_address(&self, opt_address: OptionalValue<ManagedAddress>) {
        let address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &address,
                &self.locked_asset_token_id().get(),
                [EsdtLocalRole::NftBurn][..].iter().cloned(),
            )
            .async_call_and_exit()
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

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<ManagedBuffer>;

    #[view(getInitEpoch)]
    #[storage_mapper("init_epoch")]
    fn init_epoch(&self) -> SingleValueMapper<Epoch>;

    #[storage_mapper("whitelist")]
    fn whitelisted_contracts(&self) -> SetMapper<ManagedAddress>;

    #[view(getWhitelistedContracts)]
    fn get_whitelisted_contracts(&self) -> MultiValueEncoded<ManagedAddress> {
        let mut result = MultiValueEncoded::new();
        for pair in self.whitelisted_contracts().iter() {
            result.push(pair);
        }
        result
    }

    #[view(getDefaultUnlockPeriod)]
    #[storage_mapper("default_unlock_period")]
    fn default_unlock_period(&self) -> SingleValueMapper<UnlockPeriod<Self::Api>>;
}
