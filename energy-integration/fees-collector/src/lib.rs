#![no_std]

use common_structs::Percent;
use common_types::{PaymentsVec, Week};
use multiversx_sc::storage::StorageKey;
use weekly_rewards_splitting::USER_MAX_CLAIM_WEEKS;

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

    // Do not ever use these keys again!
    //
    // The whole upgrade logic can be removed after one release and upgrade on mainnet
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
        self.move_fees_current_week_after_upgrade(
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
    }

    fn move_fees_current_week_after_upgrade(
        &self,
        base_token_id: &TokenIdentifier,
        locked_token_id: &TokenIdentifier,
        all_tokens: &ManagedVec<TokenIdentifier>,
        current_week: Week,
    ) {
        let known_tokens_mapper =
            WhitelistMapper::<Self::Api, TokenIdentifier>::new(StorageKey::new(b"knownTokens"));

        for token_id in all_tokens {
            known_tokens_mapper.remove(&token_id);

            if &token_id == base_token_id || &token_id == locked_token_id {
                continue;
            }

            let acc_fees_mapper = self.accumulated_fees(current_week, &token_id);
            let acc_fees_current_week = acc_fees_mapper.get();
            if acc_fees_current_week == 0 {
                continue;
            }

            acc_fees_mapper.clear();
            self.all_accumulated_tokens(&token_id)
                .set(acc_fees_current_week);
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
            if &token_id == base_token_id && &token_id == locked_token_id {
                continue;
            }

            let balance = self
                .blockchain()
                .get_esdt_balance(&sc_address, &token_id, 0);
            if balance == 0 {
                continue;
            }

            self.all_accumulated_tokens(&token_id)
                .update(|token_balance| *token_balance += balance);
        }

        for week in (current_week - USER_MAX_CLAIM_WEEKS)..current_week {
            let remaining_rewards_mapper = self.remaining_rewards(week);
            let remaining_rewards = remaining_rewards_mapper.get();
            let opt_remaining_rewards_base_token =
                self.find_token_in_payments_vec(base_token_id, &remaining_rewards);
            let opt_remaining_rewards_locked_token =
                self.find_token_in_payments_vec(locked_token_id, &remaining_rewards);

            let mut new_remaining_rewards = PaymentsVec::new();
            if let Some(remaining_rewards_base_token) = opt_remaining_rewards_base_token {
                new_remaining_rewards.push(remaining_rewards_base_token);
            }
            if let Some(remaining_rewards_locked_token) = opt_remaining_rewards_locked_token {
                new_remaining_rewards.push(remaining_rewards_locked_token);
            }

            remaining_rewards_mapper.set(new_remaining_rewards);
        }
    }

    fn find_token_in_payments_vec(
        &self,
        token_id: &TokenIdentifier,
        vec: &PaymentsVec<Self::Api>,
    ) -> Option<EsdtTokenPayment> {
        for payment in vec {
            if token_id == &payment.token_identifier {
                return Some(payment);
            }
        }

        None
    }

    // only needed for testing the upgrade functionality

    #[storage_mapper("allTokens")]
    fn all_tokens(&self) -> SingleValueMapper<ManagedVec<TokenIdentifier>>;
}
