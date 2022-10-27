elrond_wasm::imports!();

use farm::{ClaimRewardsResultType, EnterFarmResultType, ExitFarmResultType, ProxyTrait as _};

pub struct EnterFarmResultWrapper<M: ManagedTypeApi> {
    pub farm_token: EsdtTokenPayment<M>,
    pub reward_token: EsdtTokenPayment<M>,
}

pub struct ExitFarmResultWrapper<M: ManagedTypeApi> {
    pub farming_tokens: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
    pub remaining_farm_tokens: EsdtTokenPayment<M>,
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
        let original_caller = self.blockchain().get_caller();
        let enter_farm_result: EnterFarmResultType<Self::Api> = self
            .farm_contract_proxy(farm_address)
            .enter_farm_endpoint(original_caller)
            .add_esdt_token_transfer(farming_token_id, 0, farming_token_amount)
            .execute_on_dest_context();

        let (output_farm_token_payment, rewards_payment) = enter_farm_result.clone().into_tuple();

        EnterFarmResultWrapper {
            farm_token: output_farm_token_payment,
            reward_token: rewards_payment,
        }
    }

    fn call_exit_farm(
        &self,
        farm_address: ManagedAddress,
        farm_token: EsdtTokenPayment,
        exit_amount: BigUint,
    ) -> ExitFarmResultWrapper<Self::Api> {
        let original_caller = self.blockchain().get_caller();
        let raw_result: ExitFarmResultType<Self::Api> = self
            .farm_contract_proxy(farm_address)
            .exit_farm_endpoint(exit_amount, original_caller)
            .add_esdt_token_transfer(
                farm_token.token_identifier,
                farm_token.token_nonce,
                farm_token.amount,
            )
            .execute_on_dest_context();
        let (farming_tokens, reward_tokens, remaining_farm_tokens) = raw_result.into_tuple();

        ExitFarmResultWrapper {
            farming_tokens,
            reward_tokens,
            remaining_farm_tokens,
        }
    }

    fn call_claim_rewards_farm(
        &self,
        farm_address: ManagedAddress,
        farm_token: EsdtTokenPayment,
    ) -> ClaimRewardsFarmResultWrapper<Self::Api> {
        let original_caller = self.blockchain().get_caller();
        let raw_result: ClaimRewardsResultType<Self::Api> = self
            .farm_contract_proxy(farm_address)
            .claim_rewards_endpoint(original_caller)
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
        let original_caller = self.blockchain().get_caller();
        let new_farm_token = self
            .farm_contract_proxy(farm_address)
            .compound_rewards_endpoint(original_caller)
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
