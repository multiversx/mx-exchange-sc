use common_structs::{RawResultWrapper, RawResultsType};

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type EnterFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiValue3<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
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
        remaining_farm_tokens: EsdtTokenPayment<M>,
    ) -> Self;

    fn get_initial_farming_tokens(&self) -> EsdtTokenPayment<M>;

    fn get_reward_tokens(&self) -> EsdtTokenPayment<M>;

    fn get_remaining_farm_tokens(&self) -> EsdtTokenPayment<M>;
}

impl<M: ManagedTypeApi> ExitFarmResult<M> for ExitFarmResultWrapper<M> {
    fn new(
        initial_farming_tokens: EsdtTokenPayment<M>,
        reward_tokens: EsdtTokenPayment<M>,
        remaining_farm_tokens: EsdtTokenPayment<M>,
    ) -> Self {
        ExitFarmResultWrapper {
            initial_farming_tokens,
            reward_tokens,
            remaining_farm_tokens,
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
    fn get_remaining_farm_tokens(&self) -> EsdtTokenPayment<M> {
        self.remaining_farm_tokens.clone()
    }
}

mod old_farm_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait FarmProxy {
        #[payable("*")]
        #[endpoint(exitFarm)]
        fn exit_farm(&self) -> MultiValueEncoded<EsdtTokenPayment>;
    }
}

mod farm_proxy {
    elrond_wasm::imports!();
    use super::{ClaimRewardsResultType, EnterFarmResultType, ExitFarmResultType};

    #[elrond_wasm::proxy]
    pub trait FarmProxy {
        #[payable("*")]
        #[endpoint(enterFarm)]
        fn enter_farm(
            &self,
            opt_orig_caller: OptionalValue<ManagedAddress>,
        ) -> EnterFarmResultType<Self::Api>;

        #[payable("*")]
        #[endpoint(exitFarm)]
        fn exit_farm(
            &self,
            exit_amount: BigUint,
            opt_orig_caller: OptionalValue<ManagedAddress>,
        ) -> ExitFarmResultType<Self::Api>;

        #[payable("*")]
        #[endpoint(claimRewards)]
        fn claim_rewards(
            &self,
            opt_orig_caller: OptionalValue<ManagedAddress>,
        ) -> ClaimRewardsResultType<Self::Api>;
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
        caller: ManagedAddress,
    ) -> EnterFarmResultWrapper<Self::Api> {
        let mut contract_call = self
            .farm_proxy(farm_address)
            .enter_farm(caller)
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
        caller: ManagedAddress,
    ) -> ExitFarmResultWrapper<Self::Api> {
        let additional_results = usize::from(opt_exit_amount.is_some());

        let raw_results: RawResultsType<Self::Api> = match opt_exit_amount {
            OptionalValue::Some(exit_amount) => self
                .farm_proxy(farm_address)
                .exit_farm(exit_amount, caller)
                .with_esdt_transfer((farm_token.clone(), farm_token_nonce, farm_token_amount))
                .execute_on_dest_context(),
            OptionalValue::None => self
                .old_farm_proxy(farm_address)
                .exit_farm()
                .with_esdt_transfer((farm_token.clone(), farm_token_nonce, farm_token_amount))
                .execute_on_dest_context(),
        };

        let mut results_wrapper = RawResultWrapper::new(raw_results);
        results_wrapper.trim_results_front(EXIT_FARM_BASE_RESULTS_LEN + additional_results);

        let initial_farming_tokens = results_wrapper.decode_next_result();
        let reward_tokens = results_wrapper.decode_next_result();
        let remaining_farm_tokens = if additional_results == 1 {
            results_wrapper.decode_next_result()
        } else {
            EsdtTokenPayment::new(farm_token, farm_token_nonce, BigUint::zero())
        };

        ExitFarmResultWrapper {
            initial_farming_tokens,
            reward_tokens,
            remaining_farm_tokens,
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
        let raw_results: RawResultsType<Self::Api> = self
            .farm_proxy(farm_address)
            .claim_rewards(caller)
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

    #[proxy]
    fn old_farm_proxy(&self, sc_address: ManagedAddress) -> old_farm_proxy::Proxy<Self::Api>;
}
