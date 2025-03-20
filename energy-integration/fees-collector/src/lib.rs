#![no_std]

use common_structs::Percent;
use multiversx_sc::storage::StorageKey;

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

        let known_tokens_mapper =
            WhitelistMapper::<Self::Api, TokenIdentifier>::new(StorageKey::new(b"knownTokens"));

        let base_token_id = self.get_base_token_id();
        let locked_token_id = self.get_locked_token_id();
        let current_week = self.get_current_week();
        for token_id in &all_tokens {
            known_tokens_mapper.remove(&token_id);

            if token_id == base_token_id || token_id == locked_token_id {
                continue;
            }

            let acc_fees_current_week = self.accumulated_fees(current_week, &token_id).take();
            if acc_fees_current_week == 0 {
                continue;
            }

            self.all_accumulated_tokens(&token_id)
                .set(acc_fees_current_week);
        }
    }
}
