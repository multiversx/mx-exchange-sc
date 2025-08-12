#![no_std]

multiversx_sc::imports!();

use common_structs::Percent;
use multiversx_sc::storage::StorageKey;

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

        self.set_base_reward_tokens();

        for admin in admins {
            self.add_admin(admin);
        }
    }

    #[upgrade]
    fn upgrade(&self, blocks_per_epoch_opt: OptionalValue<u64>) {
        // Legacy storage
        let locked_token_id_mapper =
            SingleValueMapper::<Self::Api, TokenIdentifier>::new(StorageKey::new(b"lockedTokenId"));
        locked_token_id_mapper.clear();

        // Migrate existing data to new storage structure
        let all_tokens_mapper = SingleValueMapper::<Self::Api, ManagedVec<TokenIdentifier>>::new(
            StorageKey::new(b"allTokens"),
        );
        let all_tokens = all_tokens_mapper.take();

        let known_tokens_mapper =
            WhitelistMapper::<Self::Api, TokenIdentifier>::new(StorageKey::new(b"knownTokens"));
        let mut reward_tokens_mapper = self.reward_tokens();

        for token_id in &all_tokens {
            known_tokens_mapper.remove(&token_id);
            reward_tokens_mapper.insert(token_id);
        }

        let locked_tokens_per_block_mapper =
            SingleValueMapper::<Self::Api, BigUint>::new(StorageKey::new(b"lockedTokensPerBlock"));
        let locked_tokens_per_block = locked_tokens_per_block_mapper.take();
        let mut blocks_per_epoch = 10u64 * 60u64 * 24u64; // 14400 blocks per epoch
       let blocks_per_epoch = match blocks_per_epoch_opt {
           OptionalValue::Some(blocks_per_epoch_new) => blocks_per_epoch_new,
           OptionalValue::None => 10u64 * 60u64 * 24u64
        };
        let locked_tokens_per_epoch = locked_tokens_per_block * blocks_per_epoch;

        self.locked_tokens_per_epoch()
            .set_if_empty(locked_tokens_per_epoch);
    }
}
