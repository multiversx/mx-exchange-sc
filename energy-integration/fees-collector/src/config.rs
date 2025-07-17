multiversx_sc::imports!();

use common_structs::{Percent, Week};
use energy_factory::lock_options::MAX_PENALTY_PERCENTAGE;

#[multiversx_sc::module]
pub trait ConfigModule:
    energy_query::EnergyQueryModule + week_timekeeping::WeekTimekeepingModule
{
    #[only_owner]
    #[endpoint(addKnownContracts)]
    fn add_known_contracts(&self, contracts: MultiValueEncoded<ManagedAddress>) {
        let mut mapper = self.known_contracts();
        for sc in contracts {
            require!(
                self.blockchain().is_smart_contract(&sc),
                "Invalid SC address"
            );

            let _ = mapper.insert(sc);
        }
    }

    #[only_owner]
    #[endpoint(removeKnownContracts)]
    fn remove_known_contracts(&self, contracts: MultiValueEncoded<ManagedAddress>) {
        let mut mapper = self.known_contracts();
        for sc in contracts {
            let _ = mapper.swap_remove(&sc);
        }
    }

    #[only_owner]
    #[endpoint(setBaseTokenBurnPercent)]
    fn set_base_token_burn_percent(&self, burn_percent: Percent) {
        require!(burn_percent <= MAX_PENALTY_PERCENTAGE, "Invalid percent");

        self.base_token_burn_percent().set(burn_percent);
    }

    #[only_owner]
    #[endpoint(removeRewardTokens)]
    fn remove_reward_tokens(&self, token_ids: MultiValueEncoded<TokenIdentifier>) {
        let locked_token_id = self.get_locked_token_id();
        let base_token_id = self.get_base_token_id();

        for token_id in token_ids {
            require!(
                token_id != locked_token_id && token_id != base_token_id,
                "Cannot remove locked or base token"
            );
            require!(
                self.reward_tokens().swap_remove(&token_id),
                "Token not found"
            );

            let current_week = self.get_current_week();
            self.accumulated_fees(current_week, &token_id).clear();
        }
    }

    fn set_base_reward_tokens(&self) {
        let locked_token_id = self.get_locked_token_id();
        let base_token_id = self.get_base_token_id();

        self.reward_tokens().insert(locked_token_id);
        self.reward_tokens().insert(base_token_id);
    }

    #[view(getRewardTokens)]
    #[storage_mapper("rewardTokens")]
    fn reward_tokens(&self) -> UnorderedSetMapper<TokenIdentifier>;

    #[view(getAllKnownContracts)]
    #[storage_mapper("knownContracts")]
    fn known_contracts(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[view(getAccumulatedFees)]
    #[storage_mapper("accumulatedFees")]
    fn accumulated_fees(&self, week: Week, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    #[storage_mapper("baseTokenBurnPercent")]
    fn base_token_burn_percent(&self) -> SingleValueMapper<Percent>;

    // Update for this storage disabled for this version of the exchange
    #[view(getAllowExternalClaimRewards)]
    #[storage_mapper("allowExternalClaimRewards")]
    fn allow_external_claim_rewards(&self, user: &ManagedAddress) -> SingleValueMapper<bool>;
}
