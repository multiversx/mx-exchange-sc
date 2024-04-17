multiversx_sc::imports!();

use farm::{
    base_functions::{ClaimRewardsResultType, ClaimRewardsResultWrapper},
    EnterFarmResultType, ExitFarmWithPartialPosResultType,
};
use farm_with_locked_rewards::ProxyTrait as _;

pub struct EnterFarmResultWrapper<M: ManagedTypeApi> {
    pub farm_token: EsdtTokenPayment<M>,
    pub reward_token: EsdtTokenPayment<M>,
}

pub struct ExitFarmResultWrapper<M: ManagedTypeApi> {
    pub farming_tokens: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
}

#[multiversx_sc::module]
pub trait FarmInteractionsModule {
    fn call_enter_farm(
        &self,
        user: ManagedAddress,
        farm_address: ManagedAddress,
        farming_token_id: TokenIdentifier,
        farming_token_amount: BigUint,
    ) -> EnterFarmResultWrapper<Self::Api> {
        let enter_farm_result: EnterFarmResultType<Self::Api> = self
            .farm_contract_proxy(farm_address)
            .enter_farm_endpoint(user)
            .with_esdt_transfer((farming_token_id, 0, farming_token_amount))
            .execute_on_dest_context();

        let (output_farm_token_payment, rewards_payment) = enter_farm_result.into_tuple();

        EnterFarmResultWrapper {
            farm_token: output_farm_token_payment,
            reward_token: rewards_payment,
        }
    }

    fn call_exit_farm(
        &self,
        user: ManagedAddress,
        farm_address: ManagedAddress,
        farm_token: EsdtTokenPayment,
    ) -> ExitFarmResultWrapper<Self::Api> {
        let raw_result: ExitFarmWithPartialPosResultType<Self::Api> = self
            .farm_contract_proxy(farm_address)
            .exit_farm_endpoint(user)
            .with_esdt_transfer(farm_token)
            .execute_on_dest_context();
        let (farming_tokens, reward_tokens) = raw_result.into_tuple();

        ExitFarmResultWrapper {
            farming_tokens,
            reward_tokens,
        }
    }

    fn call_claim_rewards_farm(
        &self,
        user: ManagedAddress,
        farm_address: ManagedAddress,
        farm_token: EsdtTokenPayment,
    ) -> ClaimRewardsResultWrapper<Self::Api> {
        let raw_result: ClaimRewardsResultType<Self::Api> = self
            .farm_contract_proxy(farm_address)
            .claim_rewards_endpoint(user)
            .with_esdt_transfer(farm_token)
            .execute_on_dest_context();
        let (new_farm_token, rewards) = raw_result.into_tuple();

        ClaimRewardsResultWrapper {
            new_farm_token,
            rewards,
        }
    }

    #[proxy]
    fn farm_contract_proxy(&self, to: ManagedAddress)
        -> farm_with_locked_rewards::Proxy<Self::Api>;
}
