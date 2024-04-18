use crate::farm_proxy;
use common_structs::RawResultWrapper;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

const ENTER_FARM_RESULTS_LEN: usize = 2;
const EXIT_FARM_RESULTS_LEN: usize = 2;
const CLAIM_REWARDS_RESULTS_LEN: usize = 2;

pub struct EnterFarmResultWrapper<M: ManagedTypeApi> {
    pub farm_tokens: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
}

pub struct ExitFarmResultWrapper<M: ManagedTypeApi> {
    pub initial_farming_tokens: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
}

pub struct FarmClaimRewardsResultWrapper<M: ManagedTypeApi> {
    pub new_farm_tokens: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
}

pub struct FarmCompoundRewardsResultWrapper<M: ManagedTypeApi> {
    pub new_farm_tokens: EsdtTokenPayment<M>,
}

#[multiversx_sc::module]
pub trait FarmInteractionsModule {
    fn call_farm_enter(
        &self,
        farm_address: ManagedAddress,
        farming_token: TokenIdentifier,
        farming_token_amount: BigUint,
        additional_farm_tokens: ManagedVec<EsdtTokenPayment<Self::Api>>,
        caller: ManagedAddress,
    ) -> EnterFarmResultWrapper<Self::Api> {
        let mut payment = ManagedVec::from_single_item(EsdtTokenPayment::new(
            farming_token,
            0,
            farming_token_amount,
        ));
        payment.extend(&additional_farm_tokens);

        let raw_results = self
            .tx()
            .to(&farm_address)
            .typed(farm_proxy::FarmProxy)
            .enter_farm(caller)
            .payment(payment)
            .returns(ReturnsRawResult)
            .sync_call();

        let mut results_wrapper = RawResultWrapper::new(raw_results.into());
        results_wrapper.trim_results_front(ENTER_FARM_RESULTS_LEN);

        let new_farm_tokens = results_wrapper.decode_next_result();
        let reward_tokens = results_wrapper.decode_next_result();

        EnterFarmResultWrapper {
            farm_tokens: new_farm_tokens,
            reward_tokens,
        }
    }

    fn call_farm_exit(
        &self,
        farm_address: ManagedAddress,
        farm_token: TokenIdentifier,
        farm_token_nonce: u64,
        farm_token_amount: BigUint,
        caller: ManagedAddress,
    ) -> ExitFarmResultWrapper<Self::Api> {
        let raw_results = self
            .tx()
            .to(&farm_address)
            .typed(farm_proxy::FarmProxy)
            .exit_farm(caller)
            .single_esdt(&farm_token, farm_token_nonce, &farm_token_amount)
            .returns(ReturnsRawResult)
            .sync_call();

        let mut results_wrapper = RawResultWrapper::new(raw_results.into());
        results_wrapper.trim_results_front(EXIT_FARM_RESULTS_LEN);

        let initial_farming_tokens = results_wrapper.decode_next_result();
        let reward_tokens = results_wrapper.decode_next_result();

        ExitFarmResultWrapper {
            initial_farming_tokens,
            reward_tokens,
        }
    }

    fn call_farm_claim_rewards(
        &self,
        farm_address: ManagedAddress,
        farm_token: TokenIdentifier,
        farm_token_nonce: u64,
        farm_token_amount: BigUint,
        caller: ManagedAddress,
    ) -> FarmClaimRewardsResultWrapper<Self::Api> {
        let raw_results = self
            .tx()
            .to(&farm_address)
            .typed(farm_proxy::FarmProxy)
            .claim_rewards(caller)
            .single_esdt(&farm_token, farm_token_nonce, &farm_token_amount)
            .returns(ReturnsRawResult)
            .sync_call();

        let mut results_wrapper = RawResultWrapper::new(raw_results.into());
        results_wrapper.trim_results_front(CLAIM_REWARDS_RESULTS_LEN);

        let new_farm_tokens = results_wrapper.decode_next_result();
        let reward_tokens = results_wrapper.decode_next_result();

        FarmClaimRewardsResultWrapper {
            new_farm_tokens,
            reward_tokens,
        }
    }
}
