elrond_wasm::imports!();

use farm::{ClaimRewardsResultType, EnterFarmResultType, ExitFarmResultType, ProxyTrait as _};

pub struct EnterFarmResultWrapper<M: ManagedTypeApi> {
    pub farm_token: EsdtTokenPayment<M>,
}

pub struct ExitFarmResultWrapper<M: ManagedTypeApi> {
    pub farming_tokens: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
}

pub struct ClaimRewardsFarmResultWrapper<M: ManagedTypeApi> {
    pub new_farm_token: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
}

pub struct CompoundRewardsFarmResultWrapper<M: ManagedTypeApi> {
    pub new_farm_token: EsdtTokenPayment<M>,
}

#[elrond_wasm::module]
pub trait FarmInteractionsModule {
    fn call_enter_farm(
        &self,
        farm_address: ManagedAddress,
        farming_token_id: TokenIdentifier,
        farming_token_amount: BigUint,
    ) -> EnterFarmResultWrapper<Self::Api> {
        let result: EnterFarmResultType<Self::Api> = self
            .farm_contract_proxy(farm_address)
            .enter_farm()
            .add_esdt_token_transfer(farming_token_id, 0, farming_token_amount)
            .execute_on_dest_context();

        EnterFarmResultWrapper { farm_token: result }
    }

    fn call_exit_farm(
        &self,
        farm_address: ManagedAddress,
        farm_token: EsdtTokenPayment,
    ) -> ExitFarmResultWrapper<Self::Api> {
        let raw_result: ExitFarmResultType<Self::Api> = self
            .farm_contract_proxy(farm_address)
            .exit_farm()
            .add_esdt_token_transfer(
                farm_token.token_identifier,
                farm_token.token_nonce,
                farm_token.amount,
            )
            .execute_on_dest_context();
        let (farming_tokens, reward_tokens) = raw_result.into_tuple();

        ExitFarmResultWrapper {
            farming_tokens,
            reward_tokens,
        }
    }

    fn call_claim_rewards_farm(
        &self,
        farm_address: ManagedAddress,
        farm_token: EsdtTokenPayment,
    ) -> ClaimRewardsFarmResultWrapper<Self::Api> {
        let raw_result: ClaimRewardsResultType<Self::Api> = self
            .farm_contract_proxy(farm_address)
            .claim_rewards()
            .add_esdt_token_transfer(
                farm_token.token_identifier,
                farm_token.token_nonce,
                farm_token.amount,
            )
            .execute_on_dest_context();
        let (new_farm_token, reward_tokens) = raw_result.into_tuple();

        ClaimRewardsFarmResultWrapper {
            new_farm_token,
            reward_tokens,
        }
    }

    fn call_compound_rewards_farm(
        &self,
        farm_address: ManagedAddress,
        farm_token: EsdtTokenPayment,
    ) -> CompoundRewardsFarmResultWrapper<Self::Api> {
        let new_farm_token = self
            .farm_contract_proxy(farm_address)
            .compound_rewards()
            .add_esdt_token_transfer(
                farm_token.token_identifier,
                farm_token.token_nonce,
                farm_token.amount,
            )
            .execute_on_dest_context();

        CompoundRewardsFarmResultWrapper { new_farm_token }
    }

    #[proxy]
    fn farm_contract_proxy(&self, to: ManagedAddress) -> farm::Proxy<Self::Api>;
}
