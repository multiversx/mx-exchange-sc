elrond_wasm::imports!();

use core::mem::swap;

use farm::ProxyTrait as _;
use pair::safe_price::ProxyTrait as _;

use crate::result_types::*;
use farm_staking::{ClaimRewardsResultType, EnterFarmResultType, ExitFarmResultType};
use pair::RemoveLiquidityResultType;

pub type SafePriceResult<Api> = MultiValue2<EsdtTokenPayment<Api>, EsdtTokenPayment<Api>>;

#[elrond_wasm::module]
pub trait ExternalContractsInteractionsModule:
    crate::lp_farm_token::LpFarmTokenModule + token_merge_helper::TokenMergeHelperModule
{
    // lp farm

    fn lp_farm_claim_rewards(
        &self,
        lp_farm_token_id: TokenIdentifier,
        lp_farm_token_nonce: u64,
        lp_farm_token_amount: BigUint,
    ) -> LpFarmClaimRewardsResult<Self::Api> {
        let orig_caller = self.blockchain().get_caller();
        let lp_farm_address = self.lp_farm_address().get();
        let lp_farm_result: ClaimRewardsResultType<Self::Api> = self
            .lp_farm_proxy_obj(lp_farm_address)
            .claim_rewards_endpoint(orig_caller)
            .add_esdt_token_transfer(
                lp_farm_token_id.clone(),
                lp_farm_token_nonce,
                lp_farm_token_amount,
            )
            .execute_on_dest_context();
        let (mut new_lp_farm_tokens, mut lp_farm_rewards) = lp_farm_result.into_tuple();

        self.swap_payments_if_wrong_order(
            &mut new_lp_farm_tokens,
            &mut lp_farm_rewards,
            &lp_farm_token_id,
            b"lp_farm_claim_rewards",
        );

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
        let orig_caller = self.blockchain().get_caller();
        let lp_farm_token_id = self.lp_farm_token_id().get();
        let lp_farm_address = self.lp_farm_address().get();
        let exit_farm_result: ExitFarmResultType<Self::Api> = self
            .lp_farm_proxy_obj(lp_farm_address)
            .exit_farm_endpoint(orig_caller)
            .add_esdt_token_transfer(lp_farm_token_id, lp_farm_token_nonce, lp_farm_token_amount)
            .execute_on_dest_context();
        let (mut lp_tokens, mut lp_farm_rewards) = exit_farm_result.into_tuple();
        let expected_lp_token_id = self.lp_token_id().get();

        self.swap_payments_if_wrong_order(
            &mut lp_tokens,
            &mut lp_farm_rewards,
            &expected_lp_token_id,
            b"lp_farm_exit",
        );

        LpFarmExitResult {
            lp_tokens,
            lp_farm_rewards,
        }
    }

    fn merge_lp_farm_tokens(
        &self,
        caller: ManagedAddress,
        base_lp_token: EsdtTokenPayment<Self::Api>,
        mut additional_lp_tokens: ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> EsdtTokenPayment<Self::Api> {
        if additional_lp_tokens.is_empty() {
            return base_lp_token;
        }

        additional_lp_tokens.push(base_lp_token);

        let lp_farm_address = self.lp_farm_address().get();
        self.lp_farm_proxy_obj(lp_farm_address)
            .merge_farm_tokens_endpoint(OptionalValue::Some(caller))
            .with_multi_token_transfer(additional_lp_tokens)
            .execute_on_dest_context()
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
            .execute_on_dest_context();

        StakingFarmEnterResult {
            received_staking_farm_token,
        }
    }

    fn staking_farm_claim_rewards(
        &self,
        staking_farm_token_id: TokenIdentifier,
        staking_farm_token_nonce: u64,
        staking_farm_token_amount: BigUint,
        new_staking_farm_value: BigUint,
    ) -> StakingFarmClaimRewardsResult<Self::Api> {
        let staking_farm_address = self.staking_farm_address().get();
        let staking_farm_result: ClaimRewardsResultType<Self::Api> = self
            .staking_farm_proxy_obj(staking_farm_address)
            .claim_rewards_with_new_value(new_staking_farm_value)
            .add_esdt_token_transfer(
                staking_farm_token_id.clone(),
                staking_farm_token_nonce,
                staking_farm_token_amount,
            )
            .execute_on_dest_context();
        let (mut new_staking_farm_tokens, mut staking_farm_rewards) =
            staking_farm_result.into_tuple();

        self.swap_payments_if_wrong_order(
            &mut new_staking_farm_tokens,
            &mut staking_farm_rewards,
            &staking_farm_token_id,
            b"staking_farm_claim_rewards",
        );

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
            staking_farm_token_id.clone(),
            farm_token_nonce,
            farm_token_amount,
        ));

        let staking_farm_address = self.staking_farm_address().get();
        let unstake_result: ExitFarmResultType<Self::Api> = self
            .staking_farm_proxy_obj(staking_farm_address)
            .unstake_farm_through_proxy()
            .with_multi_token_transfer(payments)
            .execute_on_dest_context();
        let (mut unbond_staking_farm_token, mut staking_rewards) = unstake_result.into_tuple();

        self.swap_payments_if_wrong_order(
            &mut unbond_staking_farm_token,
            &mut staking_rewards,
            &staking_farm_token_id,
            b"staking_farm_unstake",
        );

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
            .remove_liquidity(pair_first_token_min_amount, pair_second_token_min_amount)
            .add_esdt_token_transfer(
                lp_tokens.token_identifier,
                lp_tokens.token_nonce,
                lp_tokens.amount,
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

    fn swap_payments_if_wrong_order(
        &self,
        first_payment: &mut EsdtTokenPayment<Self::Api>,
        second_payment: &mut EsdtTokenPayment<Self::Api>,
        expected_first_payment_id: &TokenIdentifier,
        called_function_name: &[u8],
    ) {
        if &first_payment.token_identifier != expected_first_payment_id {
            if &second_payment.token_identifier == expected_first_payment_id {
                swap(first_payment, second_payment);
            } else {
                sc_panic!(
                    "Invalid tokens received on {}",
                    ManagedBuffer::new_from_bytes(called_function_name)
                );
            }
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

    #[view(getLpTokenId)]
    #[storage_mapper("lpTokenId")]
    fn lp_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
