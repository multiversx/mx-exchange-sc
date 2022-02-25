elrond_wasm::imports!();

use farm::farm_token_merge::ProxyTrait as _;
use pair::safe_price::ProxyTrait as _;

use crate::result_types::*;
use farm_staking::{ClaimRewardsResultType, EnterFarmResultType, ExitFarmResultType};
use pair::RemoveLiquidityResultType;

pub type SafePriceResult<Api> = MultiValue2<EsdtTokenPayment<Api>, EsdtTokenPayment<Api>>;

#[elrond_wasm::module]
pub trait ExternalContractsInteractionsModule:
    crate::lp_farm_token::LpFarmTokenModule + token_merge::TokenMergeModule
{
    // lp farm

    fn lp_farm_claim_rewards(
        &self,
        lp_farm_tokens: PaymentsVec<Self::Api>,
    ) -> LpFarmClaimRewardsResult<Self::Api> {
        let lp_farm_address = self.lp_farm_address().get();
        let lp_farm_result: ClaimRewardsResultType<Self::Api> = self
            .lp_farm_proxy_obj(lp_farm_address)
            .claim_rewards(OptionalValue::None)
            .with_multi_token_transfer(lp_farm_tokens)
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after));
        let (new_lp_farm_tokens, lp_farm_rewards) = lp_farm_result.into_tuple();

        LpFarmClaimRewardsResult {
            new_lp_farm_tokens,
            lp_farm_rewards,
        }
    }

    fn lp_farm_exit(
        &self,
        lp_farm_token_nonce: u64,
        lp_farm_token_amount: BigUint,
    ) -> LpFarmExitResult<Self::Api> {
        let lp_farm_token_id = self.lp_farm_token_id().get();
        let lp_farm_address = self.lp_farm_address().get();
        let exit_farm_result: ExitFarmResultType<Self::Api> = self
            .lp_farm_proxy_obj(lp_farm_address)
            .exit_farm(OptionalValue::None)
            .add_token_transfer(lp_farm_token_id, lp_farm_token_nonce, lp_farm_token_amount)
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after));
        let (lp_tokens, lp_farm_rewards) = exit_farm_result.into_tuple();

        LpFarmExitResult {
            lp_tokens,
            lp_farm_rewards,
        }
    }

    fn merge_lp_farm_tokens(
        &self,
        base_lp_token: EsdtTokenPayment<Self::Api>,
        mut additional_lp_tokens: ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> EsdtTokenPayment<Self::Api> {
        if additional_lp_tokens.is_empty() {
            return base_lp_token;
        }

        additional_lp_tokens.push(base_lp_token);

        let lp_farm_address = self.lp_farm_address().get();
        self.lp_farm_proxy_obj(lp_farm_address)
            .merge_farm_tokens(OptionalValue::None)
            .with_multi_token_transfer(additional_lp_tokens)
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after))
    }

    // staking farm

    fn staking_farm_enter(
        &self,
        staking_token_amount: BigUint,
        staking_farm_tokens: PaymentsVec<Self::Api>,
    ) -> StakingFarmEnterResult<Self::Api> {
        let staking_farm_address = self.staking_farm_address().get();
        let received_staking_farm_token: EnterFarmResultType<Self::Api> = self
            .staking_farm_proxy_obj(staking_farm_address)
            .stake_farm_through_proxy(staking_token_amount)
            .with_multi_token_transfer(staking_farm_tokens)
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after));

        StakingFarmEnterResult {
            received_staking_farm_token,
        }
    }

    fn staking_farm_claim_rewards(
        &self,
        new_staking_farm_values: ManagedVec<BigUint>,
        staking_farm_tokens: PaymentsVec<Self::Api>,
    ) -> StakingFarmClaimRewardsResult<Self::Api> {
        let staking_farm_address = self.staking_farm_address().get();
        let staking_farm_result: ClaimRewardsResultType<Self::Api> = self
            .staking_farm_proxy_obj(staking_farm_address)
            .claim_rewards_with_new_value(new_staking_farm_values)
            .with_multi_token_transfer(staking_farm_tokens)
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after));
        let (new_staking_farm_tokens, staking_farm_rewards) = staking_farm_result.into_tuple();

        StakingFarmClaimRewardsResult {
            new_staking_farm_tokens,
            staking_farm_rewards,
        }
    }

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
        let unstake_result: ExitFarmResultType<Self::Api> = self
            .staking_farm_proxy_obj(staking_farm_address)
            .unstake_farm_through_proxy()
            .with_multi_token_transfer(payments)
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after));
        let (unbond_staking_farm_token, staking_rewards) = unstake_result.into_tuple();

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
        let pair_withdraw_result: RemoveLiquidityResultType<Self::Api> = self
            .pair_proxy_obj(pair_address)
            .remove_liquidity(
                lp_tokens.token_identifier,
                lp_tokens.token_nonce,
                lp_tokens.amount,
                pair_first_token_min_amount,
                pair_second_token_min_amount,
                OptionalValue::None,
            )
            .execute_on_dest_context();
        let (pair_first_token_payment, pair_second_token_payment) =
            pair_withdraw_result.into_tuple();

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
        let result: SafePriceResult<Self::Api> = self
            .pair_proxy_obj(pair_address)
            .update_and_get_tokens_for_given_position_with_safe_price(lp_tokens_amount)
            .execute_on_dest_context();
        let (first_token_info, second_token_info) = result.into_tuple();
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
    fn staking_farm_proxy_obj(&self, sc_address: ManagedAddress) -> farm_staking::Proxy<Self::Api>;

    #[proxy]
    fn lp_farm_proxy_obj(&self, sc_address: ManagedAddress) -> farm::Proxy<Self::Api>;

    #[proxy]
    fn pair_proxy_obj(&self, sc_address: ManagedAddress) -> pair::Proxy<Self::Api>;

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
