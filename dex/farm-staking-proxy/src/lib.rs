#![no_std]

elrond_wasm::imports!();

pub mod dual_yield_token;
pub mod external_contracts_interactions;
pub mod lp_farm_token;
pub mod result_types;

pub type UnstakeResult<Api> = MultiValueEncoded<Api, EsdtTokenPayment<Api>>;

#[elrond_wasm::contract]
pub trait FarmStakingProxy:
    dual_yield_token::DualYieldTokenModule
    + external_contracts_interactions::ExternalContractsInteractionsModule
    + lp_farm_token::LpFarmTokenModule
    + token_merge::TokenMergeModule
{
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint(unstakeFarmTokens)]
    fn unstake_farm_tokens(
        &self,
        pair_first_token_min_amount: BigUint,
        pair_second_token_min_amount: BigUint,
    ) -> UnstakeResult<Self::Api> {
        let (payment_token, payment_nonce, payment_amount) =
            self.call_value().single_esdt().into_tuple();
        self.require_dual_yield_token(&payment_token);

        let attributes = self.get_dual_yield_token_attributes(payment_nonce);
        let lp_farm_token_amount =
            self.get_lp_farm_token_amount_equivalent(&attributes, &payment_amount);
        let lp_farm_exit_result =
            self.lp_farm_exit(attributes.lp_farm_token_nonce, lp_farm_token_amount);

        let remove_liq_result = self.pair_remove_liquidity(
            lp_farm_exit_result.lp_tokens,
            pair_first_token_min_amount,
            pair_second_token_min_amount,
        );

        let staking_farm_token_amount =
            self.get_staking_farm_token_amount_equivalent(&payment_amount);
        let staking_farm_exit_result = self.staking_farm_unstake(
            remove_liq_result.staking_token_payment,
            attributes.staking_farm_token_nonce,
            staking_farm_token_amount,
        );
        let unstake_result = self.send_unstake_payments(
            remove_liq_result.other_token_payment,
            lp_farm_exit_result.lp_farm_rewards,
            staking_farm_exit_result.staking_rewards,
            staking_farm_exit_result.unbond_staking_farm_token,
        );

        self.burn_dual_yield_tokens(payment_nonce, &payment_amount);

        unstake_result
    }

    fn send_unstake_payments(
        &self,
        other_token_payment: EsdtTokenPayment<Self::Api>,
        lp_farm_rewards: EsdtTokenPayment<Self::Api>,
        staking_rewards: EsdtTokenPayment<Self::Api>,
        unbond_staking_farm_token: EsdtTokenPayment<Self::Api>,
    ) -> UnstakeResult<Self::Api> {
        let caller = self.blockchain().get_caller();
        let mut user_payments = ManagedVec::new();
        if other_token_payment.amount > 0 {
            user_payments.push(other_token_payment);
        }
        if lp_farm_rewards.amount > 0 {
            user_payments.push(lp_farm_rewards);
        }
        if staking_rewards.amount > 0 {
            user_payments.push(staking_rewards);
        }
        user_payments.push(unbond_staking_farm_token);

        self.send().direct_multi(&caller, &user_payments);

        user_payments.into()
    }
}
