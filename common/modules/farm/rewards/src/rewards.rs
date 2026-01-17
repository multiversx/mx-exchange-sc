#![no_std]

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait RewardsModule:
    config::ConfigModule + pausable::PausableModule + permissions_module::PermissionsModule
{
    fn start_produce_rewards(&self) {
        require!(
            self.per_second_reward_amount().get() != 0u64,
            "Cannot produce zero reward amount"
        );
        require!(
            !self.produce_rewards_enabled().get(),
            "Producing rewards is already enabled"
        );
        let current_timestamp = self.blockchain().get_block_timestamp();
        self.produce_rewards_enabled().set(true);
        self.last_reward_timestamp().set(current_timestamp);
    }

    #[inline]
    fn produces_per_second_rewards(&self) -> bool {
        self.produce_rewards_enabled().get()
    }

    #[view(getRewardPerShare)]
    #[storage_mapper("reward_per_share")]
    fn reward_per_share(&self) -> SingleValueMapper<BigUint>;

    #[view(getRewardReserve)]
    #[storage_mapper("reward_reserve")]
    fn reward_reserve(&self) -> SingleValueMapper<BigUint>;
}
