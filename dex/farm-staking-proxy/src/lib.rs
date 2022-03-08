#![no_std]

elrond_wasm::imports!();

pub mod dual_yield_token;
pub mod external_contracts_interactions;
pub mod lp_farm_token;
pub mod result_types;

pub type StakeResult<Api> = EsdtTokenPayment<Api>;
pub type ClaimDualYieldResult<Api> = MultiValueEncoded<Api, EsdtTokenPayment<Api>>;
pub type UnstakeResult<Api> = MultiValueEncoded<Api, EsdtTokenPayment<Api>>;

#[elrond_wasm::contract]
pub trait FarmStakingProxy:
    dual_yield_token::DualYieldTokenModule
    + external_contracts_interactions::ExternalContractsInteractionsModule
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
        lp_token_id: TokenIdentifier,
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
        require!(staking_token_id.is_esdt(), "Invalid Staking token ID");
        require!(lp_farm_token_id.is_esdt(), "Invalid Farm token ID");
        require!(
            staking_farm_token_id.is_esdt(),
            "Invalid Staking Farm token ID"
        );

        require!(lp_token_id.is_esdt(), "Invalide LP token ID");

        self.lp_farm_address().set(&lp_farm_address);
        self.staking_farm_address().set(&staking_farm_address);
        self.pair_address().set(&pair_address);
        self.staking_token_id().set(&staking_token_id);
        self.lp_farm_token_id().set(&lp_farm_token_id);
        self.staking_farm_token_id().set(&staking_farm_token_id);
        self.lp_token_id().set(&lp_token_id);
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

        let lp_tokens_in_farm = self.get_lp_tokens_in_farm_position(
            lp_farm_token_payment.token_nonce,
            &lp_farm_token_payment.amount,
        );
        let merged_lp_farm_tokens =
            self.merge_lp_farm_tokens(lp_farm_token_payment, additional_lp_farm_tokens);

        let staking_token_amount = self.get_lp_tokens_safe_price(lp_tokens_in_farm);
        let received_staking_farm_token = self
            .staking_farm_enter(staking_token_amount, staking_farm_tokens)
            .received_staking_farm_token;

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

        let lp_farm_claim_rewards_result = self.lp_farm_claim_rewards(lp_farm_tokens);
        let staking_farm_claim_rewards_result =
            self.staking_farm_claim_rewards(new_staking_farm_values, staking_farm_tokens);

        let new_lp_farm_tokens = lp_farm_claim_rewards_result.new_lp_farm_tokens;
        let new_staking_farm_tokens = staking_farm_claim_rewards_result.new_staking_farm_tokens;
        let new_dual_yield_tokens = self.create_dual_yield_tokens(
            new_lp_farm_tokens.token_nonce,
            new_lp_farm_tokens.amount,
            new_staking_farm_tokens.token_nonce,
            new_staking_farm_tokens.amount,
        );

        self.send_claim_payments(
            lp_farm_claim_rewards_result.lp_farm_rewards,
            staking_farm_claim_rewards_result.staking_farm_rewards,
            new_dual_yield_tokens,
        )
    }

    fn send_claim_payments(
        &self,
        lp_farm_rewards: EsdtTokenPayment<Self::Api>,
        staking_farm_rewards: EsdtTokenPayment<Self::Api>,
        new_dual_yield_tokens: EsdtTokenPayment<Self::Api>,
    ) -> ClaimDualYieldResult<Self::Api> {
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

        let _ = Self::Api::send_api_impl().direct_multi_esdt_transfer_execute(
            &caller,
            &user_payments,
            0,
            &ManagedBuffer::new(),
            &ManagedArgBuffer::new_empty(),
        );

        user_payments.into()
    }
}
