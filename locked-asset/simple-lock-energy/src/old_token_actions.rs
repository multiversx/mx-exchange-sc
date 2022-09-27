elrond_wasm::imports!();

use common_structs::{Epoch, OldLockedTokenAttributes, UnlockMilestoneEx, UnlockScheduleEx};
use factory::locked_asset::MAX_MILESTONES_IN_SCHEDULE;
use simple_lock::error_messages::CANNOT_UNLOCK_YET_ERR_MSG;

use crate::{energy::Energy, migration::OLD_TOKEN_NAME};

pub struct OldLockedUnlockedTokenPair<M: ManagedTypeApi> {
    pub opt_locked: Option<EsdtTokenPayment<M>>,
    pub unlocked: EsdtTokenPayment<M>,
}

#[elrond_wasm::module]
pub trait OldTokenActions:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::token_whitelist::TokenWhitelistModule
    + crate::lock_options::LockOptionsModule
    + crate::old_token_nonces::OldTokenNonces
    + crate::util::UtilModule
    + crate::energy::EnergyModule
    + crate::events::EventsModule
    + elrond_wasm_modules::pause::PauseModule
{
    fn unlock_old_token(
        &self,
        payment: EsdtTokenPayment,
        energy: &mut Energy<Self::Api>,
        current_epoch: Epoch,
    ) -> OldLockedUnlockedTokenPair<Self::Api> {
        let locked_token_mapper = self.locked_token();
        locked_token_mapper.require_same_token(&payment.token_identifier);

        let mut attributes: OldLockedTokenAttributes<Self::Api> =
            locked_token_mapper.get_token_attributes(payment.token_nonce);
        let unlock_epoch_amount_pairs = attributes
            .get_unlock_amounts_per_milestone::<MAX_MILESTONES_IN_SCHEDULE>(&payment.amount);
        let unlockable_entries = unlock_epoch_amount_pairs.get_unlockable_entries(current_epoch);
        let total_unlockable_entries = unlockable_entries.len();
        require!(total_unlockable_entries > 0, CANNOT_UNLOCK_YET_ERR_MSG);

        let mut unlockable_amount = BigUint::zero();
        for entry in unlockable_entries {
            energy.refund_after_token_unlock(&entry.amount, entry.epoch, current_epoch);
            unlockable_amount += entry.amount;
        }

        locked_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        let leftover_locked = &payment.amount - &unlockable_amount;
        let opt_locked = if leftover_locked > 0 {
            attributes.remove_first_milestones(total_unlockable_entries);

            let new_token_nonce = self.get_or_create_nonce_for_attributes(
                &locked_token_mapper,
                &ManagedBuffer::new_from_bytes(OLD_TOKEN_NAME),
                &attributes,
            );
            let new_tokens = locked_token_mapper.nft_add_quantity(new_token_nonce, leftover_locked);

            Some(new_tokens)
        } else {
            None
        };

        let base_asset_token_id = self.base_asset_token_id().get();
        let unlocked_tokens = EsdtTokenPayment::new(base_asset_token_id, 0, unlockable_amount);

        OldLockedUnlockedTokenPair {
            opt_locked,
            unlocked: unlocked_tokens,
        }
    }

    fn extend_old_token_period(
        &self,
        payment: EsdtTokenPayment,
        new_unlock_epoch: Epoch,
    ) -> EsdtTokenPayment {
        let locked_token_mapper = self.locked_token();
        locked_token_mapper.require_same_token(&payment.token_identifier);

        let attributes: OldLockedTokenAttributes<Self::Api> =
            locked_token_mapper.get_token_attributes(payment.token_nonce);
        let current_unlock_milestones = &attributes.unlock_schedule.unlock_milestones;
        let unlock_epoch_amount_pairs = attributes
            .get_unlock_amounts_per_milestone::<MAX_MILESTONES_IN_SCHEDULE>(&payment.amount);

        let caller = self.blockchain().get_caller();
        let current_epoch = self.blockchain().get_block_epoch();
        let mut energy = self.get_updated_energy_entry_for_user(&caller, current_epoch);

        let mut unaffected_milestones = ManagedVec::<Self::Api, UnlockMilestoneEx>::new();
        let mut aggregated_new_unlock_percent = 0;
        for (epoch_amount_pair, current_milestone) in unlock_epoch_amount_pairs
            .pairs
            .iter()
            .zip(current_unlock_milestones.iter())
        {
            if current_milestone.unlock_epoch < new_unlock_epoch {
                aggregated_new_unlock_percent += current_milestone.unlock_percent;

                energy.update_after_unlock_any(
                    &epoch_amount_pair.amount,
                    epoch_amount_pair.epoch,
                    current_epoch,
                );
                energy.add_after_token_lock(
                    &epoch_amount_pair.amount,
                    new_unlock_epoch,
                    current_epoch,
                );
            } else {
                unaffected_milestones.push(current_milestone);
            }
        }

        let extended_milestone = UnlockMilestoneEx {
            unlock_epoch: new_unlock_epoch,
            unlock_percent: aggregated_new_unlock_percent,
        };
        let mut new_unlock_milestones = ManagedVec::from_single_item(extended_milestone);
        new_unlock_milestones.append_vec(unaffected_milestones);

        let new_attributes = OldLockedTokenAttributes {
            unlock_schedule: UnlockScheduleEx {
                unlock_milestones: new_unlock_milestones,
            },
            is_merged: attributes.is_merged,
        };
        let new_token_nonce = self.get_or_create_nonce_for_attributes(
            &locked_token_mapper,
            &ManagedBuffer::new_from_bytes(OLD_TOKEN_NAME),
            &new_attributes,
        );

        locked_token_mapper.nft_add_quantity_and_send(&caller, new_token_nonce, payment.amount)
    }
}
