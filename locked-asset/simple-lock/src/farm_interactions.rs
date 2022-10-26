elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
type ExitFarmResultType<BigUint> =
    MultiValue3<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

pub struct EnterFarmResultWrapper<M: ManagedTypeApi> {
    pub farm_tokens: EsdtTokenPayment<M>,
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

mod farm_proxy {
    elrond_wasm::imports!();
    use super::{ClaimRewardsResultType, EnterFarmResultType, ExitFarmResultType};

    #[elrond_wasm::proxy]
    pub trait FarmProxy {
        #[payable("*")]
        #[endpoint(enterFarm)]
        fn enter_farm(&self) -> EnterFarmResultType<Self::Api>;

        #[payable("*")]
        #[endpoint(exitFarm)]
        fn exit_farm(&self, exit_amount: BigUint) -> ExitFarmResultType<Self::Api>;

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
            .add_esdt_token_transfer(farming_token, 0, farming_token_amount);
        for farm_token in &additional_farm_tokens {
            contract_call = contract_call.add_esdt_token_transfer(
                farm_token.token_identifier,
                farm_token.token_nonce,
                farm_token.amount,
            );
        }

        let result: MultiValueEncoded<ManagedBuffer> = contract_call.execute_on_dest_context();
        let raw = result.into_vec_of_buffers();
        require!(
            !raw.is_empty(),
            "at least one result expected from enterFarm"
        );

        let last_elem = raw.get(raw.len() - 1);
        let new_farm_tokens: EnterFarmResultType<Self::Api> =
            self.serializer().top_decode_from_managed_buffer(&last_elem);

        EnterFarmResultWrapper {
            farm_tokens: new_farm_tokens,
        }
    }

    fn call_farm_exit(
        &self,
        farm_address: ManagedAddress,
        farm_token: TokenIdentifier,
        farm_token_nonce: u64,
        farm_token_amount: BigUint,
        exit_amount: BigUint,
    ) -> ExitFarmResultWrapper<Self::Api> {
        let result: MultiValueEncoded<ManagedBuffer> = self
            .farm_proxy(farm_address)
            .exit_farm(exit_amount)
            .add_esdt_token_transfer(farm_token, farm_token_nonce, farm_token_amount)
            .execute_on_dest_context();

        let raw = result.into_vec_of_buffers();
        require!(raw.len() > 2, "at least 3 results expected from exitFarm");

        let initial_farming_tokens = self
            .serializer()
            .top_decode_from_managed_buffer(&raw.get(raw.len() - 3));
        let reward_tokens = self
            .serializer()
            .top_decode_from_managed_buffer(&raw.get(raw.len() - 2));
        let remaining_farm_tokens = self
            .serializer()
            .top_decode_from_managed_buffer(&raw.get(raw.len() - 1));

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
    ) -> FarmClaimRewardsResultWrapper<Self::Api> {
        let result: MultiValueEncoded<ManagedBuffer> = self
            .farm_proxy(farm_address)
            .claim_rewards()
            .add_esdt_token_transfer(farm_token, farm_token_nonce, farm_token_amount)
            .execute_on_dest_context();

        let raw = result.into_vec_of_buffers();
        require!(
            raw.len() > 1,
            "at least 2 results expected from claim rewards"
        );

        let new_farm_tokens = self
            .serializer()
            .top_decode_from_managed_buffer(&raw.get(raw.len() - 2));
        let reward_tokens = self
            .serializer()
            .top_decode_from_managed_buffer(&raw.get(raw.len() - 1));

        FarmClaimRewardsResultWrapper {
            new_farm_tokens,
            reward_tokens,
        }
    }

    #[proxy]
    fn farm_proxy(&self, sc_address: ManagedAddress) -> farm_proxy::Proxy<Self::Api>;
}
