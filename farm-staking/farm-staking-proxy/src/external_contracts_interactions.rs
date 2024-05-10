multiversx_sc::imports!();

use farm::base_functions::DoubleMultiPayment;

use crate::farm_staking_proxy_methods;
use crate::farm_with_locked_rewards_proxy;
use crate::pair_proxy;
use crate::result_types::*;

pub type SafePriceResult<Api> = MultiValue2<EsdtTokenPayment<Api>, EsdtTokenPayment<Api>>;

#[multiversx_sc::module]
pub trait ExternalContractsInteractionsModule:
    crate::lp_farm_token::LpFarmTokenModule + utils::UtilsModule + energy_query::EnergyQueryModule
{
    // lp farm

    fn lp_farm_claim_rewards(
        &self,
        orig_caller: ManagedAddress,
        lp_farm_token_id: TokenIdentifier,
        lp_farm_token_nonce: u64,
        lp_farm_token_amount: BigUint,
    ) -> LpFarmClaimRewardsResult<Self::Api> {
        let lp_farm_address = self.lp_farm_address().get();
        let lp_farm_result = self
            .tx()
            .to(&lp_farm_address)
            .typed(farm_with_locked_rewards_proxy::FarmProxy)
            .claim_rewards_endpoint(orig_caller)
            .single_esdt(
                &lp_farm_token_id,
                lp_farm_token_nonce,
                &lp_farm_token_amount,
            )
            .returns(ReturnsResult)
            .sync_call();

        let (new_lp_farm_tokens, lp_farm_rewards) = lp_farm_result.into_tuple();

        LpFarmClaimRewardsResult {
            new_lp_farm_tokens,
            lp_farm_rewards,
        }
    }

    fn lp_farm_exit(
        &self,
        orig_caller: ManagedAddress,
        lp_farm_token_nonce: u64,
        lp_farm_token_amount: BigUint,
    ) -> LpFarmExitResult<Self::Api> {
        let lp_farm_token_id = self.lp_farm_token_id().get();
        let lp_farm_address = self.lp_farm_address().get();
        let exit_farm_result = self
            .tx()
            .to(&lp_farm_address)
            .typed(farm_with_locked_rewards_proxy::FarmProxy)
            .exit_farm_endpoint(orig_caller)
            .single_esdt(
                &lp_farm_token_id,
                lp_farm_token_nonce,
                &lp_farm_token_amount,
            )
            .returns(ReturnsResult)
            .sync_call();

        let (lp_tokens, lp_farm_rewards) = exit_farm_result.into_tuple();

        LpFarmExitResult {
            lp_tokens,
            lp_farm_rewards,
        }
    }

    fn merge_lp_farm_tokens(
        &self,
        orig_caller: ManagedAddress,
        base_lp_farm_token: EsdtTokenPayment,
        mut additional_lp_farm_tokens: PaymentsVec<Self::Api>,
    ) -> DoubleMultiPayment<Self::Api> {
        if additional_lp_farm_tokens.is_empty() {
            let locked_token_id = self.get_locked_token_id();
            let rewards_payment = EsdtTokenPayment::new(locked_token_id, 0, BigUint::zero());
            return (base_lp_farm_token, rewards_payment).into();
        }

        additional_lp_farm_tokens.push(base_lp_farm_token);

        let lp_farm_address = self.lp_farm_address().get();
        self.tx()
            .to(lp_farm_address)
            .typed(farm_with_locked_rewards_proxy::FarmProxy)
            .merge_farm_tokens_endpoint(orig_caller)
            .payment(additional_lp_farm_tokens)
            .returns(ReturnsResult)
            .sync_call()
    }

    // staking farm

    fn staking_farm_enter(
        &self,
        orig_caller: ManagedAddress,
        staking_token_amount: BigUint,
        staking_farm_tokens: PaymentsVec<Self::Api>,
    ) -> StakingFarmEnterResult<Self::Api> {
        let staking_farm_address = self.staking_farm_address().get();
        let enter_result = self
            .tx()
            .to(&staking_farm_address)
            .typed(farm_staking_proxy_methods::FarmStakingProxy)
            .stake_farm_through_proxy(staking_token_amount, orig_caller)
            .payment(staking_farm_tokens)
            .returns(ReturnsResult)
            .sync_call();

        let (received_staking_farm_token, boosted_rewards) = enter_result.into_tuple();

        StakingFarmEnterResult {
            received_staking_farm_token,
            boosted_rewards,
        }
    }

    fn staking_farm_claim_rewards(
        &self,
        orig_caller: ManagedAddress,
        staking_farm_token_id: TokenIdentifier,
        staking_farm_token_nonce: u64,
        staking_farm_token_amount: BigUint,
        new_staking_farm_value: BigUint,
    ) -> StakingFarmClaimRewardsResult<Self::Api> {
        let staking_farm_address = self.staking_farm_address().get();
        let staking_farm_result = self
            .tx()
            .to(&staking_farm_address)
            .typed(farm_staking_proxy_methods::FarmStakingProxy)
            .claim_rewards_with_new_value(new_staking_farm_value, orig_caller)
            .single_esdt(
                &staking_farm_token_id,
                staking_farm_token_nonce,
                &staking_farm_token_amount,
            )
            .returns(ReturnsResult)
            .sync_call();

        let (new_staking_farm_tokens, staking_farm_rewards) = staking_farm_result.into_tuple();

        StakingFarmClaimRewardsResult {
            new_staking_farm_tokens,
            staking_farm_rewards,
        }
    }

    fn staking_farm_unstake(
        &self,
        orig_caller: ManagedAddress,
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
        let unstake_result = self
            .tx()
            .to(&staking_farm_address)
            .typed(farm_staking_proxy_methods::FarmStakingProxy)
            .unstake_farm_through_proxy(orig_caller)
            .payment(payments)
            .returns(ReturnsResult)
            .sync_call();

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
        let pair_withdraw_result = self
            .tx()
            .to(&pair_address)
            .typed(pair_proxy::PairProxy)
            .remove_liquidity(pair_first_token_min_amount, pair_second_token_min_amount)
            .payment(lp_tokens)
            .returns(ReturnsResult)
            .sync_call();

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
        let result = self
            .tx()
            .to(&pair_address)
            .typed(pair_proxy::PairProxy)
            .update_and_get_tokens_for_given_position_with_safe_price(lp_tokens_amount)
            .returns(ReturnsResult)
            .sync_call();

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

    #[view(getLpTokenId)]
    #[storage_mapper("lpTokenId")]
    fn lp_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
