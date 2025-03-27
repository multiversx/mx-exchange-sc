#![no_std]

use claim::FeesCollectorWrapper;
use common_structs::Percent;
use common_types::{PaymentsVec, Week};
use multiversx_sc::storage::StorageKey;
use week_timekeeping::FIRST_WEEK;
use weekly_rewards_splitting::{
    base_impl::WeeklyRewardsSplittingTraitsModule, USER_MAX_CLAIM_WEEKS,
};

multiversx_sc::imports!();

pub mod additional_locked_tokens;
pub mod claim;
pub mod config;
pub mod events;
pub mod external_sc_interactions;
pub mod fees_accumulation;
pub mod redistribute_rewards;

#[multiversx_sc::contract]
pub trait FeesCollector:
    config::ConfigModule
    + events::FeesCollectorEventsModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + weekly_rewards_splitting::global_info::WeeklyRewardsGlobalInfo
    + weekly_rewards_splitting::locked_token_buckets::WeeklyRewardsLockedTokenBucketsModule
    + weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule
    + fees_accumulation::FeesAccumulationModule
    + additional_locked_tokens::AdditionalLockedTokensModule
    + locking_module::lock_with_energy_module::LockWithEnergyModule
    + energy_query::EnergyQueryModule
    + week_timekeeping::WeekTimekeepingModule
    + multiversx_sc_modules::pause::PauseModule
    + utils::UtilsModule
    + sc_whitelist_module::SCWhitelistModule
    + multiversx_sc_modules::only_admin::OnlyAdminModule
    + claim::ClaimModule
    + redistribute_rewards::RedistributeRewardsModule
    + external_sc_interactions::router::RouterInteractionsModule
{
    /// Base token burn percent is between 0 (0%) and 10_000 (100%)
    #[init]
    fn init(
        &self,
        energy_factory_address: ManagedAddress,
        router_address: ManagedAddress,
        base_token_burn_percent: Percent,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        self.set_energy_factory_address(energy_factory_address);
        self.set_router_address(router_address);
        self.set_base_token_burn_percent(base_token_burn_percent);

        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set(current_epoch);

        let locked_token_id = self.get_locked_token_id();
        self.add_known_token(locked_token_id);

        for admin in admins {
            self.add_admin(admin);
        }
    }

    // Do not use these storage keys until upgrade: "allTokens", "knownContracts" and "knownTokens"
    //
    // The whole upgrade logic (and the relevant test) can be removed after one release and upgrade on mainnet
    #[upgrade]
    fn upgrade(&self) {
        let all_tokens_mapper = SingleValueMapper::<Self::Api, ManagedVec<TokenIdentifier>>::new(
            StorageKey::new(b"allTokens"),
        );
        let all_tokens = all_tokens_mapper.take();
        if all_tokens.is_empty() {
            return;
        }

        let mut known_contracts_mapper = UnorderedSetMapper::<Self::Api, ManagedAddress>::new(
            StorageKey::new(b"knownContracts"),
        );
        known_contracts_mapper.clear();

        let base_token_id = self.get_base_token_id();
        let locked_token_id = self.get_locked_token_id();
        let current_week = self.get_current_week();
        self.clear_fees_current_week_after_upgrade(
            &base_token_id,
            &locked_token_id,
            &all_tokens,
            current_week,
        );
        self.move_fees_previous_weeks_after_upgrade(
            &base_token_id,
            &locked_token_id,
            &all_tokens,
            current_week,
        );
        self.clear_older_undist_rewards(current_week);
    }

    fn clear_fees_current_week_after_upgrade(
        &self,
        base_token_id: &TokenIdentifier,
        locked_token_id: &TokenIdentifier,
        all_tokens: &ManagedVec<TokenIdentifier>,
        current_week: Week,
    ) {
        // In case the upgrade action is the very first action in the week
        self.accumulate_additional_locked_tokens();

        let wrapper = FeesCollectorWrapper::new();
        let _ = wrapper.collect_and_get_rewards_for_week(self, current_week - 1);

        let known_tokens_mapper =
            WhitelistMapper::<Self::Api, TokenIdentifier>::new(StorageKey::new(b"knownTokens"));

        for token_id in all_tokens {
            known_tokens_mapper.remove(&token_id);

            if &token_id != base_token_id && &token_id != locked_token_id {
                self.accumulated_fees(current_week, &token_id).clear();
            }
        }
    }

    fn move_fees_previous_weeks_after_upgrade(
        &self,
        base_token_id: &TokenIdentifier,
        locked_token_id: &TokenIdentifier,
        all_tokens: &ManagedVec<TokenIdentifier>,
        current_week: Week,
    ) {
        let sc_address = self.blockchain().get_sc_address();
        for token_id in all_tokens {
            if &token_id == base_token_id || &token_id == locked_token_id {
                continue;
            }

            let balance = self
                .blockchain()
                .get_esdt_balance(&sc_address, &token_id, 0);
            if balance > 0 {
                self.all_accumulated_tokens(&token_id).set(balance);
            }
        }

        let first_week = if current_week > USER_MAX_CLAIM_WEEKS {
            current_week - USER_MAX_CLAIM_WEEKS
        } else {
            1
        };

        for week in first_week..current_week {
            self.set_rewards_after_upgrade(
                base_token_id,
                locked_token_id,
                &self.total_rewards_for_week(week),
            );
            self.set_rewards_after_upgrade(
                base_token_id,
                locked_token_id,
                &self.remaining_rewards(week),
            );
        }
    }

    fn set_rewards_after_upgrade(
        &self,
        base_token_id: &TokenIdentifier,
        locked_token_id: &TokenIdentifier,
        mapper: &SingleValueMapper<PaymentsVec<Self::Api>>,
    ) {
        let prev_rewards = mapper.get();
        let opt_rewards_base_token = self.find_token_in_payments_vec(base_token_id, &prev_rewards);
        let opt_rewards_locked_token =
            self.find_token_in_payments_vec(locked_token_id, &prev_rewards);

        let mut new_rewards = PaymentsVec::new();
        if let Some(rewards_base_token) = opt_rewards_base_token {
            new_rewards.push(rewards_base_token);
        }
        if let Some(rewards_locked_token) = opt_rewards_locked_token {
            new_rewards.push(rewards_locked_token);
        }

        mapper.set(new_rewards);
    }

    #[inline]
    fn find_token_in_payments_vec(
        &self,
        token_id: &TokenIdentifier,
        vec: &PaymentsVec<Self::Api>,
    ) -> Option<EsdtTokenPayment> {
        vec.into_iter()
            .find(|payment| token_id == &payment.token_identifier)
    }

    // This makes sure we don't accidentally redistribute rewards again
    fn clear_older_undist_rewards(&self, current_week: Week) {
        if current_week <= USER_MAX_CLAIM_WEEKS {
            return;
        }

        let end_week = current_week - USER_MAX_CLAIM_WEEKS;
        for week in FIRST_WEEK..end_week {
            self.total_rewards_for_week(week).clear();
            self.remaining_rewards(week).clear();
        }
    }

    // only needed for testing the upgrade functionality

    #[storage_mapper("allTokens")]
    fn all_tokens(&self) -> SingleValueMapper<ManagedVec<TokenIdentifier>>;
}
