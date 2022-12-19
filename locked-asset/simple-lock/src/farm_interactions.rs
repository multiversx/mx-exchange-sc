use common_structs::{RawResultWrapper, RawResultsType};

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type EnterFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

const ENTER_FARM_RESULTS_LEN: usize = 2;
const EXIT_FARM_BASE_RESULTS_LEN: usize = 2;
const CLAIM_REWARDS_RESULTS_LEN: usize = 2;

pub struct EnterFarmResultWrapper<M: ManagedTypeApi> {
    pub farm_tokens: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
}

pub struct ExitFarmResultWrapper<M: ManagedTypeApi> {
    pub initial_farming_tokens: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
    pub remaining_farm_tokens: EsdtTokenPayment<M>,
}

pub struct FarmClaimRewardsResultWrapper<M: ManagedTypeApi> {
    pub new_farm_tokens: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
}

pub struct FarmCompoundRewardsResultWrapper<M: ManagedTypeApi> {
    pub new_farm_tokens: EsdtTokenPayment<M>,
}

pub trait ExitFarmResult<M: ManagedTypeApi> {
    fn new(
        initial_farming_tokens: EsdtTokenPayment<M>,
        reward_tokens: EsdtTokenPayment<M>,
        additional_tokens: ManagedVec<M, EsdtTokenPayment<M>>,
    ) -> Self;

    fn get_initial_farming_tokens(&self) -> EsdtTokenPayment<M>;

    fn get_reward_tokens(&self) -> EsdtTokenPayment<M>;

    fn get_additional_results_expected_len() -> usize;

    fn get_additional_tokens(&self) -> ManagedVec<M, EsdtTokenPayment<M>>;
}

impl<M: ManagedTypeApi> ExitFarmResult<M> for ExitFarmResultWrapper<M> {
    fn new(
        initial_farming_tokens: EsdtTokenPayment<M>,
        reward_tokens: EsdtTokenPayment<M>,
        additional_tokens: ManagedVec<M, EsdtTokenPayment<M>>,
    ) -> Self {
        ExitFarmResultWrapper {
            initial_farming_tokens,
            reward_tokens,
            remaining_farm_tokens: additional_tokens.get(0),
        }
    }

    #[inline]
    fn get_initial_farming_tokens(&self) -> EsdtTokenPayment<M> {
        self.initial_farming_tokens.clone()
    }

    #[inline]
    fn get_reward_tokens(&self) -> EsdtTokenPayment<M> {
        self.reward_tokens.clone()
    }

    #[inline]
    fn get_additional_results_expected_len() -> usize {
        1
    }

    fn get_additional_tokens(&self) -> ManagedVec<M, EsdtTokenPayment<M>> {
        if self.remaining_farm_tokens.amount == 0 {
            return ManagedVec::new();
        }

        ManagedVec::from_single_item(self.remaining_farm_tokens.clone())
    }
}

mod farm_proxy {
    elrond_wasm::imports!();
    use super::{ClaimRewardsResultType, EnterFarmResultType};

    #[elrond_wasm::proxy]
    pub trait FarmProxy {
        #[payable("*")]
        #[endpoint(enterFarm)]
        fn enter_farm(&self) -> EnterFarmResultType<Self::Api>;

        #[payable("*")]
        #[endpoint(exitFarm)]
        fn exit_farm(
            &self,
            exit_amount: OptionalValue<BigUint>,
        ) -> MultiValueEncoded<EsdtTokenPayment>;

        #[payable("*")]
        #[endpoint(claimRewards)]
        fn claim_rewards(&self) -> ClaimRewardsResultType<Self::Api>;
    }
}

#[elrond_wasm::module]
pub trait FarmInteractionsModule {
    fn call_farm_enter(
        &self,
        farm_address: ManagedAddress,
        farming_token: TokenIdentifier,
        farming_token_amount: BigUint,
        additional_farm_tokens: ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> EnterFarmResultWrapper<Self::Api> {
        let mut contract_call = self
            .farm_proxy(farm_address)
            .enter_farm()
            .with_esdt_transfer((farming_token, 0, farming_token_amount));
        for farm_token in &additional_farm_tokens {
            contract_call = contract_call.with_esdt_transfer(farm_token);
        }

        let raw_results: RawResultsType<Self::Api> = contract_call.execute_on_dest_context();
        let mut results_wrapper = RawResultWrapper::new(raw_results);
        results_wrapper.trim_results_front(ENTER_FARM_RESULTS_LEN);

        let new_farm_tokens = results_wrapper.decode_next_result();
        let reward_tokens = results_wrapper.decode_next_result();

        EnterFarmResultWrapper {
            farm_tokens: new_farm_tokens,
            reward_tokens,
        }
    }

    fn call_farm_exit<ResultsType: ExitFarmResult<Self::Api>>(
        &self,
        farm_address: ManagedAddress,
        farm_token: TokenIdentifier,
        farm_token_nonce: u64,
        farm_token_amount: BigUint,
        opt_exit_amount: OptionalValue<BigUint>,
    ) -> ResultsType {
        let raw_results: RawResultsType<Self::Api> = self
            .farm_proxy(farm_address)
            .exit_farm(opt_exit_amount)
            .with_esdt_transfer((farm_token, farm_token_nonce, farm_token_amount))
            .execute_on_dest_context();

        let mut results_wrapper = RawResultWrapper::new(raw_results);
        let additional_results_len = ResultsType::get_additional_results_expected_len();
        results_wrapper.trim_results_front(EXIT_FARM_BASE_RESULTS_LEN + additional_results_len);

        let initial_farming_tokens = results_wrapper.decode_next_result();
        let reward_tokens = results_wrapper.decode_next_result();

        let mut additional_tokens = ManagedVec::new();
        for _ in 0..additional_results_len {
            let additional_token = results_wrapper.decode_next_result();
            additional_tokens.push(additional_token);
        }

        ResultsType::new(initial_farming_tokens, reward_tokens, additional_tokens)
    }

    fn call_farm_claim_rewards(
        &self,
        farm_address: ManagedAddress,
        farm_token: TokenIdentifier,
        farm_token_nonce: u64,
        farm_token_amount: BigUint,
    ) -> FarmClaimRewardsResultWrapper<Self::Api> {
        let raw_results: RawResultsType<Self::Api> = self
            .farm_proxy(farm_address)
            .claim_rewards()
            .with_esdt_transfer((farm_token, farm_token_nonce, farm_token_amount))
            .execute_on_dest_context();

        let mut results_wrapper = RawResultWrapper::new(raw_results);
        results_wrapper.trim_results_front(CLAIM_REWARDS_RESULTS_LEN);

        let new_farm_tokens = results_wrapper.decode_next_result();
        let reward_tokens = results_wrapper.decode_next_result();

        FarmClaimRewardsResultWrapper {
            new_farm_tokens,
            reward_tokens,
        }
    }

    #[proxy]
    fn farm_proxy(&self, sc_address: ManagedAddress) -> farm_proxy::Proxy<Self::Api>;
}
