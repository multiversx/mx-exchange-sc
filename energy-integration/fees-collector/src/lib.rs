#![no_std]

multiversx_sc::imports!();

pub mod additional_locked_tokens;
pub mod claim;
pub mod config;
pub mod events;
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
{
    #[init]
    fn init(
        &self,
        locked_token_id: TokenIdentifier,
        energy_factory_address: ManagedAddress,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        self.require_valid_token_id(&locked_token_id);
        self.require_sc_address(&energy_factory_address);

        let current_epoch = self.blockchain().get_block_epoch();
        self.first_week_start_epoch().set(current_epoch);

        let mut tokens = MultiValueEncoded::new();
        tokens.push(locked_token_id.clone());
        self.add_known_tokens(tokens);

        self.locked_token_id().set(locked_token_id);
        self.energy_factory_address().set(energy_factory_address);

        for admin in admins {
            self.add_admin(admin);
        }
    }

    #[upgrade]
    fn upgrade(&self) {}
}
