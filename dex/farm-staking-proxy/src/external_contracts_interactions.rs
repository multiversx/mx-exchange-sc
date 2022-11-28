elrond_wasm::imports!();

use common_structs::{RawResultWrapper, RawResultsType};

use crate::result_types::*;

mod farm_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait FarmProxy {
        #[payable("*")]
        #[endpoint(exitFarm)]
        fn exit_farm(&self) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment>;

        #[view(getFarmingTokenId)]
        fn farming_token_id(&self) -> TokenIdentifier;
    }
}

mod farm_staking_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait FarmStakingProxy {
        #[payable("*")]
        #[endpoint(unstakeFarmThroughProxy)]
        fn unstake_farm_through_proxy(&self) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment>;
    }
}

mod pair_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait PairProxy {
        #[payable("*")]
        #[endpoint(removeLiquidity)]
        fn remove_liquidity(
            &self,
            first_token_amount_min: BigUint,
            second_token_amount_min: BigUint,
        ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment>;

        #[endpoint(updateAndGetTokensForGivenPositionWithSafePrice)]
        fn update_and_get_tokens_for_given_position_with_safe_price(
            &self,
            liquidity: BigUint,
        ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment>;
    }
}

#[elrond_wasm::module]
pub trait ExternalContractsInteractionsModule:
    crate::lp_farm_token::LpFarmTokenModule + token_merge::TokenMergeModule
{
    // lp farm

    fn lp_farm_exit(
        &self,
        lp_farm_token_nonce: u64,
        lp_farm_token_amount: BigUint,
    ) -> LpFarmExitResult<Self::Api> {
        let lp_farm_token_id = self.lp_farm_token_id().get();
        let lp_farm_address = self.lp_farm_address().get();
        let raw_results: RawResultsType<Self::Api> = self
            .lp_farm_proxy_obj(lp_farm_address)
            .exit_farm()
            .add_esdt_token_transfer(lp_farm_token_id, lp_farm_token_nonce, lp_farm_token_amount)
            .execute_on_dest_context();

        let mut results_wrapper = RawResultWrapper::new(raw_results);
        results_wrapper.trim_results_front(2);

        let mut lp_tokens: EsdtTokenPayment = results_wrapper.decode_next_result();
        let mut lp_farm_rewards: EsdtTokenPayment = results_wrapper.decode_next_result();

        let received_lp_token_identifier = lp_tokens.token_identifier.clone();
        let lp_token_identifier = self.get_lp_farming_token_identifier();

        if lp_token_identifier != received_lp_token_identifier {
            core::mem::swap(&mut lp_tokens, &mut lp_farm_rewards);
        }

        LpFarmExitResult {
            lp_tokens,
            lp_farm_rewards,
        }
    }

    fn get_lp_farming_token_identifier(&self) -> TokenIdentifier {
        let lp_farm_address = self.lp_farm_address().get();
        self.lp_farm_proxy_obj(lp_farm_address)
            .farming_token_id()
            .execute_on_dest_context()
    }

    // staking farm

    fn staking_farm_unstake(
        &self,
        staking_tokens: EsdtTokenPayment<Self::Api>,
        farm_token_nonce: u64,
        farm_token_amount: BigUint,
    ) -> StakingFarmExitResult<Self::Api> {
        let staking_farm_token_id = self.staking_farm_token_id().get();
        let mut payments = ManagedVec::from_single_item(staking_tokens);
        payments.push(EsdtTokenPayment::new(
            staking_farm_token_id,
            farm_token_nonce,
            farm_token_amount,
        ));

        let staking_farm_address = self.staking_farm_address().get();
        let raw_results: RawResultsType<Self::Api> = self
            .staking_farm_proxy_obj(staking_farm_address)
            .unstake_farm_through_proxy()
            .with_multi_token_transfer(payments)
            .execute_on_dest_context();

        let mut results_wrapper = RawResultWrapper::new(raw_results);
        results_wrapper.trim_results_front(2);

        let unbond_staking_farm_token = results_wrapper.decode_next_result();
        let staking_rewards = results_wrapper.decode_next_result();

        StakingFarmExitResult {
            unbond_staking_farm_token,
            staking_rewards,
        }
    }

    // pair

    fn pair_remove_liquidity(
        &self,
        lp_tokens: EsdtTokenPayment<Self::Api>,
        pair_first_token_min_amount: BigUint,
        pair_second_token_min_amount: BigUint,
    ) -> PairRemoveLiquidityResult<Self::Api> {
        let pair_address = self.pair_address().get();
        let raw_results: RawResultsType<Self::Api> = self
            .pair_proxy_obj(pair_address)
            .remove_liquidity(pair_first_token_min_amount, pair_second_token_min_amount)
            .add_esdt_token_transfer(
                lp_tokens.token_identifier,
                lp_tokens.token_nonce,
                lp_tokens.amount,
            )
            .execute_on_dest_context();

        let mut results_wrapper = RawResultWrapper::new(raw_results);
        results_wrapper.trim_results_front(2);

        let pair_first_token_payment: EsdtTokenPayment = results_wrapper.decode_next_result();
        let pair_second_token_payment: EsdtTokenPayment = results_wrapper.decode_next_result();

        let staking_token_id = self.staking_token_id().get();
        let (staking_token_payment, other_token_payment) =
            if pair_first_token_payment.token_identifier == staking_token_id {
                (pair_first_token_payment, pair_second_token_payment)
            } else if pair_second_token_payment.token_identifier == staking_token_id {
                (pair_second_token_payment, pair_first_token_payment)
            } else {
                sc_panic!("Invalid payments received from Pair");
            };

        PairRemoveLiquidityResult {
            staking_token_payment,
            other_token_payment,
        }
    }

    fn get_lp_tokens_safe_price(&self, lp_tokens_amount: BigUint) -> BigUint {
        let pair_address = self.pair_address().get();
        let raw_results: RawResultsType<Self::Api> = self
            .pair_proxy_obj(pair_address)
            .update_and_get_tokens_for_given_position_with_safe_price(lp_tokens_amount)
            .execute_on_dest_context();

        let mut results_wrapper = RawResultWrapper::new(raw_results);
        results_wrapper.trim_results_front(2);

        let first_token_info: EsdtTokenPayment = results_wrapper.decode_next_result();
        let second_token_info: EsdtTokenPayment = results_wrapper.decode_next_result();

        let staking_token_id = self.staking_token_id().get();
        if first_token_info.token_identifier == staking_token_id {
            first_token_info.amount
        } else if second_token_info.token_identifier == staking_token_id {
            second_token_info.amount
        } else {
            sc_panic!("Invalid Pair contract called");
        }
    }

    // proxies

    #[proxy]
    fn staking_farm_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> farm_staking_proxy::Proxy<Self::Api>;

    #[proxy]
    fn lp_farm_proxy_obj(&self, sc_address: ManagedAddress) -> farm_proxy::Proxy<Self::Api>;

    #[proxy]
    fn pair_proxy_obj(&self, sc_address: ManagedAddress) -> pair_proxy::Proxy<Self::Api>;

    // storage

    #[view(getLpFarmAddress)]
    #[storage_mapper("lpFarmAddress")]
    fn lp_farm_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getStakingFarmAddress)]
    #[storage_mapper("stakingFarmAddress")]
    fn staking_farm_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getPairAddress)]
    #[storage_mapper("pairAddress")]
    fn pair_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getStakingTokenId)]
    #[storage_mapper("stakingTokenId")]
    fn staking_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getFarmTokenId)]
    #[storage_mapper("farmTokenId")]
    fn staking_farm_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
