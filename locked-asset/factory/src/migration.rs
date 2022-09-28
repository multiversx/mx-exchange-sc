elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait LockedTokenMigrationModule:
    crate::locked_asset::LockedAssetModule
    + token_send::TokenSendModule
    + crate::attr_ex_helper::AttrExHelper
{
    /// The new factory will need the burn role for the migrated tokens
    #[only_owner]
    #[endpoint(setLockedTokenBurnRoleForAddress)]
    fn set_locked_token_burn_role_for_address(&self, address: ManagedAddress) {
        self.locked_asset_token().set_local_roles_for_address(
            &address,
            &[EsdtLocalRole::NftBurn],
            None,
        );
    }

    /// Converts old tokens from the locked asset factory into the new version.
    /// Additionally, it also updates the user's energy accordingly.
    ///
    /// Expected payments: old LOCKED tokens
    ///
    /// Output payments: New version of the locked tokens
    #[payable("*")]
    #[endpoint(migrateTokens)]
    fn migrate_tokens(&self) -> PaymentsVec<Self::Api> {
        self.require_not_paused();

        let payments = self.call_value().all_esdt_transfers();
        require!(!payment.is_empty(), NO_PAYMENT_ERR_MSG);

        self.require_is_base_asset_token(&payment.token_identifier);

        let locked_token_mapper = self.locked_token();
        let base_asset_token_id = self.base_asset_token_id().get();

        let current_epoch = self.blockchain().get_block_epoch();
        let caller = self.blockchain().get_caller();
        let mut energy = self.get_updated_energy_entry_for_user(&caller, current_epoch);

        let mut total_tokens_in_pairs = BigUint::zero();
        let mut total_unlockable_tokens = BigUint::zero();
        let mut output_payments = ManagedVec::new();
        for pair in amount_attributes_pairs {
            let (token_amount, mut attributes) = pair.into_tuple();
            total_tokens_in_pairs += &token_amount;

            let unlock_amounts_per_epoch = attributes
                .get_unlock_amounts_per_milestone::<MAX_MILESTONES_IN_SCHEDULE>(&token_amount);

            let mut leftover_locked_amount = BigUint::zero();
            let mut total_unlockable_entries = 0;
            for epoch_amount_pair in unlock_amounts_per_epoch.pairs {
                if epoch_amount_pair.epoch > current_epoch {
                    energy.add_after_token_lock(
                        &epoch_amount_pair.amount,
                        epoch_amount_pair.epoch,
                        current_epoch,
                    );

                    leftover_locked_amount += epoch_amount_pair.amount;
                } else {
                    total_unlockable_tokens += epoch_amount_pair.amount;
                    total_unlockable_entries += 1;
                }
            }

            if leftover_locked_amount > 0 {
                attributes.remove_first_milestones(total_unlockable_entries);

                let new_locked_tokens = self.create_old_token(
                    &locked_token_mapper,
                    leftover_locked_amount,
                    &attributes,
                );
                output_payments.push(new_locked_tokens);
            }
        }

        require!(
            payment.amount == total_tokens_in_pairs,
            "Total amount mismatch"
        );

        if total_unlockable_tokens > 0 {
            let unlockable_tokens_payment =
                EsdtTokenPayment::new(base_asset_token_id, 0, total_unlockable_tokens);
            output_payments.push(unlockable_tokens_payment);
        }

        if !output_payments.is_empty() {
            self.send().direct_multi(&caller, &output_payments);
        }

        self.set_energy_entry(&caller, energy);

        output_payments
    }

    fn require_old_energy_not_updated(&self, user: &ManagedAddress) {
        require!(
            !self.user_updated_old_tokens_energy().contains(user),
            "Already updated energy"
        );
    }
}
