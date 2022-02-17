#![no_std]

elrond_wasm::imports!();

use farm::farm_token_merge::ProxyTrait as _;
use pair::safe_price::ProxyTrait as _;

use dual_yield_token::DualYieldTokenAttributes;
use farm_staking::{ClaimRewardsResultType, EnterFarmResultType, ExitFarmResultType};
use pair::RemoveLiquidityResultType;

pub mod dual_yield_token;
pub mod lp_farm_token;

pub type SafePriceResult<Api> = MultiResult2<EsdtTokenPayment<Api>, EsdtTokenPayment<Api>>;
pub type StakeResult<Api> = EsdtTokenPayment<Api>;
pub type ClaimDualYieldResult<Api> = ManagedMultiResultVec<Api, EsdtTokenPayment<Api>>;
pub type UnstakeResult<Api> = ManagedMultiResultVec<Api, EsdtTokenPayment<Api>>;

#[elrond_wasm::contract]
pub trait FarmStakingProxy:
    dual_yield_token::DualYieldTokenModule
    + lp_farm_token::LpFarmTokenModule
    + token_merge::TokenMergeModule
{
    #[init]
    fn init(
        &self,
        lp_farm_address: ManagedAddress,
        staking_farm_address: ManagedAddress,
        pair_address: ManagedAddress,
        staking_token_id: TokenIdentifier,
        lp_farm_token_id: TokenIdentifier,
        staking_farm_token_id: TokenIdentifier,
    ) {
        require!(
            self.blockchain().is_smart_contract(&lp_farm_address),
            "Invalid LP Farm address"
        );
        require!(
            self.blockchain().is_smart_contract(&staking_farm_address),
            "Invalid Staking Farm address"
        );
        require!(
            self.blockchain().is_smart_contract(&pair_address),
            "Invalid Pair address"
        );
        require!(
            staking_token_id.is_valid_esdt_identifier(),
            "Invalid Staking token ID"
        );
        require!(
            lp_farm_token_id.is_valid_esdt_identifier(),
            "Invalid LP token ID"
        );
        require!(
            staking_farm_token_id.is_valid_esdt_identifier(),
            "Invalid Staking Farm token ID"
        );

        self.lp_farm_address().set(&lp_farm_address);
        self.staking_farm_address().set(&staking_farm_address);
        self.pair_address().set(&pair_address);
        self.staking_token_id().set(&staking_token_id);
        self.lp_farm_token_id().set(&lp_farm_token_id);
        self.staking_farm_token_id().set(&staking_farm_token_id);
    }

    #[payable("*")]
    #[endpoint(stakeFarmTokens)]
    fn stake_farm_tokens(&self) -> StakeResult<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        let lp_farm_token_payment: EsdtTokenPayment<Self::Api> = payments
            .try_get(0)
            .unwrap_or_else(|| sc_panic!("empty payments"));
        let additional_payments = payments.slice(1, payments.len()).unwrap_or_default();

        let lp_farm_token_id = self.lp_farm_token_id().get();
        require!(
            lp_farm_token_payment.token_identifier == lp_farm_token_id,
            "Invalid first payment"
        );
        self.require_all_payments_dual_yield_tokens(&additional_payments);

        let staking_farm_token_id = self.staking_farm_token_id().get();
        let mut staking_farm_tokens = ManagedVec::new();
        let mut additional_lp_farm_tokens = ManagedVec::new();
        for p in &additional_payments {
            let attributes = self.get_dual_yield_token_attributes(p.token_nonce);

            staking_farm_tokens.push(EsdtTokenPayment::new(
                staking_farm_token_id.clone(),
                attributes.staking_farm_token_nonce,
                self.get_staking_farm_token_amount_equivalent(&p.amount),
            ));

            additional_lp_farm_tokens.push(EsdtTokenPayment::new(
                lp_farm_token_id.clone(),
                attributes.lp_farm_token_nonce,
                self.get_lp_farm_token_amount_equivalent(&attributes, &p.amount),
            ));

            self.burn_dual_yield_tokens(p.token_nonce, &p.amount);
        }

        let merged_lp_farm_tokens =
            self.merge_lp_farm_tokens(lp_farm_token_payment, additional_lp_farm_tokens);
        let lp_tokens_in_farm = self.get_lp_tokens_in_farm_position(
            merged_lp_farm_tokens.token_nonce,
            &merged_lp_farm_tokens.amount,
        );
        let staking_token_amount = self.get_lp_tokens_safe_price(lp_tokens_in_farm);
        let staking_farm_address = self.staking_farm_address().get();
        let received_staking_farm_token: EnterFarmResultType<Self::Api> = self
            .staking_farm_proxy_obj(staking_farm_address)
            .stake_farm_through_proxy(staking_token_amount)
            .with_multi_token_transfer(staking_farm_tokens)
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after));

        let caller = self.blockchain().get_caller();
        self.create_and_send_dual_yield_tokens(
            &caller,
            merged_lp_farm_tokens.token_nonce,
            merged_lp_farm_tokens.amount,
            received_staking_farm_token.token_nonce,
            received_staking_farm_token.amount,
        )
    }

    #[payable("*")]
    #[endpoint(claimDualYield)]
    fn claim_dual_yield(&self) -> ClaimDualYieldResult<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        self.require_all_payments_dual_yield_tokens(&payments);

        let mut lp_farm_tokens = ManagedVec::new();
        let mut staking_farm_tokens = ManagedVec::new();
        let mut new_staking_farm_values = ManagedVec::new();

        let lp_farm_token_id = self.lp_farm_token_id().get();
        let staking_farm_token_id = self.staking_farm_token_id().get();

        for p in &payments {
            let attributes = self.get_dual_yield_token_attributes(p.token_nonce);
            let staking_farm_token_amount =
                self.get_staking_farm_token_amount_equivalent(&p.amount);

            staking_farm_tokens.push(EsdtTokenPayment::new(
                staking_farm_token_id.clone(),
                attributes.staking_farm_token_nonce,
                staking_farm_token_amount,
            ));

            let lp_farm_token_amount =
                self.get_lp_farm_token_amount_equivalent(&attributes, &p.amount);
            let lp_tokens_in_position = self.get_lp_tokens_in_farm_position(
                attributes.lp_farm_token_nonce,
                &lp_farm_token_amount,
            );
            let new_staking_farm_value = self.get_lp_tokens_safe_price(lp_tokens_in_position);

            lp_farm_tokens.push(EsdtTokenPayment::new(
                lp_farm_token_id.clone(),
                attributes.lp_farm_token_nonce,
                lp_farm_token_amount,
            ));
            new_staking_farm_values.push(new_staking_farm_value);

            self.burn_dual_yield_tokens(p.token_nonce, &p.amount);
        }

        let lp_farm_address = self.lp_farm_address().get();
        let lp_farm_result: ClaimRewardsResultType<Self::Api> = self
            .lp_farm_proxy_obj(lp_farm_address)
            .claim_rewards(OptionalArg::None)
            .with_multi_token_transfer(lp_farm_tokens)
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after));
        let (new_lp_farm_tokens, lp_farm_rewards) = lp_farm_result.into_tuple();

        let staking_farm_address = self.staking_farm_address().get();
        let staking_farm_result: ClaimRewardsResultType<Self::Api> = self
            .staking_farm_proxy_obj(staking_farm_address)
            .claim_rewards_with_new_value(new_staking_farm_values)
            .with_multi_token_transfer(staking_farm_tokens)
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after));
        let (new_staking_farm_tokens, staking_farm_rewards) = staking_farm_result.into_tuple();

        let new_dual_yield_tokens = self.create_dual_yield_tokens(
            new_lp_farm_tokens.token_nonce,
            new_lp_farm_tokens.amount,
            new_staking_farm_tokens.token_nonce,
            new_staking_farm_tokens.amount,
        );

        let mut user_output_payments = ManagedVec::new();
        if lp_farm_rewards.amount > 0 {
            user_output_payments.push(lp_farm_rewards);
        }
        if staking_farm_rewards.amount > 0 {
            user_output_payments.push(staking_farm_rewards);
        }
        user_output_payments.push(new_dual_yield_tokens);

        let caller = self.blockchain().get_caller();
        let _ = Self::Api::send_api_impl().direct_multi_esdt_transfer_execute(
            &caller,
            &user_output_payments,
            0,
            &ManagedBuffer::new(),
            &ManagedArgBuffer::new_empty(),
        );

        user_output_payments.into()
    }

    #[payable("*")]
    #[endpoint(unstakeFarmTokens)]
    fn unstake_farm_tokens(
        &self,
        pair_first_token_min_amount: BigUint,
        pair_second_token_min_amount: BigUint,
    ) -> UnstakeResult<Self::Api> {
        let (payment_amount, payment_token) = self.call_value().payment_token_pair();
        let payment_nonce = self.call_value().esdt_token_nonce();

        self.require_dual_yield_token(&payment_token);

        let attributes = self.get_dual_yield_token_attributes(payment_nonce);

        let (lp_tokens, lp_farm_rewards) = self.exit_farm(&payment_amount, &attributes);

        let (staking_token_payment, other_token_payment) = self.remove_liquidity(
            lp_tokens,
            pair_first_token_min_amount,
            pair_second_token_min_amount,
        );
        let unstake_result = self.unstake(
            &payment_amount,
            &attributes,
            lp_farm_rewards,
            staking_token_payment,
            other_token_payment,
        );

        self.burn_dual_yield_tokens(payment_nonce, &payment_amount);

        unstake_result
    }

    fn exit_farm(
        &self,
        payment_amount: &BigUint,
        attributes: &DualYieldTokenAttributes<Self::Api>,
    ) -> (EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>) {
        let lp_farm_token_id = self.lp_farm_token_id().get();
        let lp_farm_token_amount =
            self.get_lp_farm_token_amount_equivalent(attributes, payment_amount);
        let lp_farm_address = self.lp_farm_address().get();
        let exit_farm_result: ExitFarmResultType<Self::Api> = self
            .lp_farm_proxy_obj(lp_farm_address)
            .exit_farm(OptionalArg::None)
            .add_token_transfer(
                lp_farm_token_id,
                attributes.lp_farm_token_nonce,
                lp_farm_token_amount,
            )
            .execute_on_dest_context();

        exit_farm_result.into_tuple()
    }

    fn remove_liquidity(
        &self,
        lp_tokens: EsdtTokenPayment<Self::Api>,
        pair_first_token_min_amount: BigUint,
        pair_second_token_min_amount: BigUint,
    ) -> (EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>) {
        let pair_address = self.pair_address().get();
        let pair_withdraw_result: RemoveLiquidityResultType<Self::Api> = self
            .pair_proxy_obj(pair_address)
            .remove_liquidity(
                lp_tokens.token_identifier,
                lp_tokens.token_nonce,
                lp_tokens.amount,
                pair_first_token_min_amount,
                pair_second_token_min_amount,
                OptionalArg::None,
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

        (staking_token_payment, other_token_payment)
    }

    fn unstake(
        &self,
        payment_amount: &BigUint,
        attributes: &DualYieldTokenAttributes<Self::Api>,
        lp_farm_rewards: EsdtTokenPayment<Self::Api>,
        staking_token_payment: EsdtTokenPayment<Self::Api>,
        other_token_payment: EsdtTokenPayment<Self::Api>,
    ) -> UnstakeResult<Self::Api> {
        let staking_farm_token_id = self.staking_farm_token_id().get();
        let staking_farm_token_amount =
            self.get_staking_farm_token_amount_equivalent(payment_amount);
        let mut staking_sc_payments = ManagedVec::new();
        staking_sc_payments.push(staking_token_payment);
        staking_sc_payments.push(EsdtTokenPayment::new(
            staking_farm_token_id,
            attributes.staking_farm_token_nonce,
            staking_farm_token_amount,
        ));

        let staking_farm_address = self.staking_farm_address().get();
        let unstake_result: ExitFarmResultType<Self::Api> = self
            .staking_farm_proxy_obj(staking_farm_address)
            .unstake_farm_through_proxy()
            .with_multi_token_transfer(staking_sc_payments)
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after));
        let (unbond_staking_farm_token, staking_rewards) = unstake_result.into_tuple();

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

        let _ = Self::Api::send_api_impl().direct_multi_esdt_transfer_execute(
            &caller,
            &user_payments,
            0,
            &ManagedBuffer::new(),
            &ManagedArgBuffer::new_empty(),
        );

        user_payments.into()
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
            .merge_farm_tokens(OptionalArg::None)
            .with_multi_token_transfer(additional_lp_tokens)
            .execute_on_dest_context()
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
