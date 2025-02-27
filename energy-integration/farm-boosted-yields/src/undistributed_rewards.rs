use common_types::Week;
use pair::pair_actions::swap::ProxyTrait as _;
use router::factory::ProxyTrait as _;
use week_timekeeping::FIRST_WEEK;
use weekly_rewards_splitting::USER_MAX_CLAIM_WEEKS;

multiversx_sc::imports!();

mod energy_factory_proxy_send_rew {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait EnergyFactorySendRewProxy {
        #[endpoint(transferUnlockedToken)]
        fn transfer_unlocked_token(&self, dest: ManagedAddress, amount: BigUint);
    }
}

#[multiversx_sc::module]
pub trait UndistributedRewardsModule:
    config::ConfigModule
    + week_timekeeping::WeekTimekeepingModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
{
    #[only_owner]
    #[endpoint(setMultisigAddress)]
    fn set_multisig_address(&self, multisig_address: ManagedAddress) {
        self.require_sc_address(&multisig_address);

        self.multisig_address().set(multisig_address);
    }

    #[only_owner]
    #[endpoint(setRouterAddress)]
    fn set_router_address(&self, router_address: ManagedAddress) {
        self.require_sc_address(&router_address);

        self.router_address().set(router_address);
    }

    #[only_owner]
    #[endpoint(collectUndistributedBoostedRewards)]
    fn collect_undistributed_boosted_rewards(
        &self,
        opt_start_week: OptionalValue<Week>,
    ) -> BigUint {
        require!(
            !self.multisig_address().is_empty(),
            "No multisig address set"
        );

        let collect_rewards_offset = USER_MAX_CLAIM_WEEKS + 1;
        let current_week = self.get_current_week();
        require!(
            current_week > collect_rewards_offset,
            "Current week must be higher than the week offset"
        );

        let end_week = current_week - collect_rewards_offset;
        let start_week = match opt_start_week {
            OptionalValue::Some(start_week) => {
                require!(start_week <= end_week, "Invalid week numbers");

                start_week
            }
            OptionalValue::None => FIRST_WEEK,
        };

        let mut total_rewards = BigUint::zero();
        for week in start_week..=end_week {
            let rewards_to_distribute = self.remaining_boosted_rewards_to_distribute(week).take();
            total_rewards += rewards_to_distribute;
        }

        if total_rewards == 0 {
            return total_rewards;
        }

        let base_token_id = self.get_base_token_id();
        let reward_token_id = self.reward_token_id().get();
        if base_token_id != reward_token_id {
            total_rewards = self.try_swap(base_token_id, reward_token_id, total_rewards);
        }

        self.send_rewards_to_multisig(total_rewards.clone());

        total_rewards
    }

    fn try_swap(
        &self,
        base_token_id: TokenIdentifier,
        reward_token_id: TokenIdentifier,
        tokens_amount: BigUint,
    ) -> BigUint {
        require!(!self.router_address().is_empty(), "No router address set");

        let router_address = self.router_address().get();
        let pair_address: ManagedAddress = self
            .router_proxy(router_address)
            .get_pair(base_token_id.clone(), reward_token_id.clone())
            .execute_on_dest_context();
        require!(!pair_address.is_zero(), "No pair found");

        let received_tokens: EsdtTokenPayment = self
            .pair_proxy(pair_address)
            .swap_tokens_fixed_input(base_token_id, BigUint::from(1u32))
            .single_esdt(&reward_token_id, 0, &tokens_amount)
            .execute_on_dest_context();

        received_tokens.amount
    }

    fn send_rewards_to_multisig(&self, total_rewards: BigUint) {
        let multisig_address = self.multisig_address().get();
        let energy_factory = self.energy_factory_address().get();
        self.energy_factory_send_rew_proxy_obj(energy_factory)
            .transfer_unlocked_token(multisig_address, total_rewards)
            .execute_on_dest_context()
    }

    #[view(getRemainingBoostedRewardsToDistribute)]
    #[storage_mapper("remainingBoostedRewardsToDistribute")]
    fn remaining_boosted_rewards_to_distribute(&self, week: Week) -> SingleValueMapper<BigUint>;

    #[storage_mapper("multisigAddress")]
    fn multisig_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("routerAddress")]
    fn router_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[proxy]
    fn energy_factory_send_rew_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> energy_factory_proxy_send_rew::Proxy<Self::Api>;

    #[proxy]
    fn router_proxy(&self, sc_address: ManagedAddress) -> router::Proxy<Self::Api>;

    #[proxy]
    fn pair_proxy(&self, sc_address: ManagedAddress) -> pair::Proxy<Self::Api>;
}
