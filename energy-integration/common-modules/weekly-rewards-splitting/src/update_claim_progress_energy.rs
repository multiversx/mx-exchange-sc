multiversx_sc::imports!();

use common_types::Week;
use energy_query::Energy;

use crate::ClaimProgress;

pub static ERROR_ENERGY_UPDATE_SAME_WEEK: &[u8] = b"Can update only after claim rewards";

#[multiversx_sc::module]
pub trait UpdateClaimProgressEnergyModule:
    energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
    + crate::events::WeeklyRewardsSplittingEventsModule
    + crate::global_info::WeeklyRewardsGlobalInfo
    + crate::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
{
    #[endpoint(updateEnergyForUser)]
    fn update_energy_for_user(&self, user: ManagedAddress) {
        let current_week = self.get_current_week();
        let claim_progress_mapper = self.current_claim_progress(&user);
        if !claim_progress_mapper.is_empty() {
            let claim_progress = claim_progress_mapper.get();
            require!(
                claim_progress.week == current_week,
                ERROR_ENERGY_UPDATE_SAME_WEEK
            );
        }
        self.update_energy_and_progress(&user);
    }

    fn update_energy_and_progress(&self, caller: &ManagedAddress) {
        let current_week = self.get_current_week();
        let current_user_energy = self.get_energy_entry(caller);

        let progress_mapper = self.current_claim_progress(caller);
        let opt_progress_for_update = if !progress_mapper.is_empty() {
            Some(progress_mapper.get())
        } else {
            None
        };
        self.update_user_energy_for_current_week(
            caller,
            current_week,
            &current_user_energy,
            opt_progress_for_update,
        );

        if current_user_energy.get_energy_amount() > 0 {
            progress_mapper.set(&ClaimProgress {
                week: current_week,
                energy: current_user_energy,
            });
        } else {
            progress_mapper.clear();
        }
    }

    fn update_user_energy_for_current_week(
        &self,
        user: &ManagedAddress,
        current_week: Week,
        current_energy: &Energy<Self::Api>,
        opt_existing_claim_progres: Option<ClaimProgress<Self::Api>>,
    ) {
        let (last_active_week, prev_energy) = match opt_existing_claim_progres {
            Some(existing_claim_progress) => {
                (existing_claim_progress.week, existing_claim_progress.energy)
            }
            None => (0, Energy::default()),
        };

        self.update_global_amounts_for_current_week(
            current_week,
            last_active_week,
            &prev_energy,
            current_energy,
        );

        self.emit_update_user_energy_event(user, current_week, current_energy);
    }

    fn clear_user_energy(
        &self,
        user: &ManagedAddress,
        remaining_farm_payment_amount: &BigUint,
        min_farm_amount: &BigUint,
    ) {
        if remaining_farm_payment_amount >= min_farm_amount {
            return;
        }

        let current_week = self.get_current_week();
        let current_epoch = self.blockchain().get_block_epoch();
        let current_user_energy = Energy::new_zero_energy(current_epoch);

        let progress_mapper = self.current_claim_progress(user);
        let opt_progress_for_update = if !progress_mapper.is_empty() {
            Some(progress_mapper.get())
        } else {
            None
        };
        self.update_user_energy_for_current_week(
            user,
            current_week,
            &current_user_energy,
            opt_progress_for_update,
        );

        progress_mapper.clear();
    }

    #[view(getCurrentClaimProgress)]
    #[storage_mapper("currentClaimProgress")]
    fn current_claim_progress(
        &self,
        user: &ManagedAddress,
    ) -> SingleValueMapper<ClaimProgress<Self::Api>>;
}
