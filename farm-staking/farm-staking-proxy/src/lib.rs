#![no_std]

use fixed_supply_token::FixedSupplyToken;
use result_types::{ClaimDualYieldResult, StakeProxyResult, UnstakeResult};

use crate::dual_yield_token::DualYieldTokenAttributes;

multiversx_sc::imports!();

pub mod dual_yield_token;
pub mod external_contracts_interactions;
pub mod lp_farm_token;
pub mod result_types;

#[multiversx_sc::contract]
pub trait FarmStakingProxy:
    dual_yield_token::DualYieldTokenModule
    + external_contracts_interactions::ExternalContractsInteractionsModule
    + lp_farm_token::LpFarmTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
    + token_send::TokenSendModule
    + sc_whitelist_module::SCWhitelistModule
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
        self.require_sc_address(&lp_farm_address);
        self.require_sc_address(&staking_farm_address);
        self.require_sc_address(&pair_address);

        self.require_valid_token_id(&staking_token_id);
        self.require_valid_token_id(&lp_farm_token_id);
        self.require_valid_token_id(&staking_farm_token_id);
        self.require_valid_token_id(&lp_token_id);

        self.lp_farm_address().set_if_empty(&lp_farm_address);
        self.staking_farm_address()
            .set_if_empty(&staking_farm_address);
        self.pair_address().set_if_empty(&pair_address);

        self.staking_token_id().set_if_empty(&staking_token_id);
        self.lp_farm_token_id().set_if_empty(&lp_farm_token_id);
        self.staking_farm_token_id()
            .set_if_empty(&staking_farm_token_id);
        self.lp_token_id().set_if_empty(&lp_token_id);
    }

    #[payable("*")]
    #[endpoint(stakeFarmTokens)]
    fn stake_farm_tokens(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> StakeProxyResult<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);
        let payments = self.get_non_empty_payments();
        let lp_farm_token_payment = payments.get(0);
        let additional_payments = payments.slice(1, payments.len()).unwrap_or_default();

        let lp_farm_token_id = self.lp_farm_token_id().get();
        require!(
            lp_farm_token_payment.token_identifier == lp_farm_token_id,
            "Invalid first payment"
        );

        let dual_yield_token_mapper = self.dual_yield_token();
        dual_yield_token_mapper.require_all_same_token(&additional_payments);

        let staking_farm_token_id = self.staking_farm_token_id().get();
        let mut additional_staking_farm_tokens = ManagedVec::new();
        let mut additional_lp_farm_tokens = ManagedVec::new();
        for p in &additional_payments {
            let attributes: DualYieldTokenAttributes<Self::Api> =
                self.get_attributes_as_part_of_fixed_supply(&p, &dual_yield_token_mapper);

            additional_staking_farm_tokens.push(EsdtTokenPayment::new(
                staking_farm_token_id.clone(),
                attributes.staking_farm_token_nonce,
                attributes.staking_farm_token_amount,
            ));

            additional_lp_farm_tokens.push(EsdtTokenPayment::new(
                lp_farm_token_id.clone(),
                attributes.lp_farm_token_nonce,
                attributes.lp_farm_token_amount,
            ));

            dual_yield_token_mapper.nft_burn(p.token_nonce, &p.amount);
        }

        let lp_tokens_in_farm = self.get_lp_tokens_in_farm_position(
            lp_farm_token_payment.token_nonce,
            &lp_farm_token_payment.amount,
        );
        let staking_token_amount = self.get_lp_tokens_safe_price(lp_tokens_in_farm);
        let staking_farm_enter_result = self.staking_farm_enter(
            orig_caller.clone(),
            staking_token_amount,
            additional_staking_farm_tokens,
        );
        let received_staking_farm_token = staking_farm_enter_result.received_staking_farm_token;

        let merged_lp_farm_tokens = self.merge_lp_farm_tokens(
            orig_caller,
            lp_farm_token_payment,
            additional_lp_farm_tokens,
        );

        let new_dual_yield_tokens = self.create_dual_yield_tokens(
            &dual_yield_token_mapper,
            merged_lp_farm_tokens.token_nonce,
            merged_lp_farm_tokens.amount,
            received_staking_farm_token.token_nonce,
            received_staking_farm_token.amount,
        );
        let output_payments = StakeProxyResult {
            dual_yield_tokens: new_dual_yield_tokens,
            boosted_rewards: staking_farm_enter_result.boosted_rewards,
        };

        output_payments.send_and_return(self, &caller)
    }

    #[payable("*")]
    #[endpoint(claimDualYield)]
    fn claim_dual_yield(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> ClaimDualYieldResult<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);
        let payment = self.call_value().single_esdt();
        let dual_yield_token_mapper = self.dual_yield_token();
        dual_yield_token_mapper.require_same_token(&payment.token_identifier);

        let attributes: DualYieldTokenAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(&payment, &dual_yield_token_mapper);

        let lp_tokens_in_position = self.get_lp_tokens_in_farm_position(
            attributes.lp_farm_token_nonce,
            &attributes.lp_farm_token_amount,
        );
        let new_staking_farm_value = self.get_lp_tokens_safe_price(lp_tokens_in_position);

        let staking_farm_token_id = self.staking_farm_token_id().get();
        let lp_farm_token_id = self.lp_farm_token_id().get();
        let lp_farm_claim_rewards_result = self.lp_farm_claim_rewards(
            orig_caller.clone(),
            lp_farm_token_id,
            attributes.lp_farm_token_nonce,
            attributes.lp_farm_token_amount,
        );
        let staking_farm_claim_rewards_result = self.staking_farm_claim_rewards(
            orig_caller,
            staking_farm_token_id,
            attributes.staking_farm_token_nonce,
            attributes.staking_farm_token_amount,
            new_staking_farm_value,
        );

        let new_lp_farm_tokens = lp_farm_claim_rewards_result.new_lp_farm_tokens;
        let new_staking_farm_tokens = staking_farm_claim_rewards_result.new_staking_farm_tokens;
        let new_dual_yield_tokens = self.create_dual_yield_tokens(
            &dual_yield_token_mapper,
            new_lp_farm_tokens.token_nonce,
            new_lp_farm_tokens.amount,
            new_staking_farm_tokens.token_nonce,
            new_staking_farm_tokens.amount,
        );

        dual_yield_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        let caller = self.blockchain().get_caller();
        let claim_result = ClaimDualYieldResult {
            lp_farm_rewards: lp_farm_claim_rewards_result.lp_farm_rewards,
            staking_farm_rewards: staking_farm_claim_rewards_result.staking_farm_rewards,
            new_dual_yield_tokens,
        };

        claim_result.send_and_return(self, &caller)
    }

    #[payable("*")]
    #[endpoint(unstakeFarmTokens)]
    fn unstake_farm_tokens(
        &self,
        pair_first_token_min_amount: BigUint,
        pair_second_token_min_amount: BigUint,
        exit_amount: BigUint,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> UnstakeResult<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);
        let payment = self.call_value().single_esdt();
        let dual_yield_token_mapper = self.dual_yield_token();
        dual_yield_token_mapper.require_same_token(&payment.token_identifier);

        let full_attributes: DualYieldTokenAttributes<Self::Api> =
            dual_yield_token_mapper.get_token_attributes(payment.token_nonce);
        let total_for_nonce = full_attributes.get_total_supply();
        require!(
            payment.amount == total_for_nonce,
            "Must exit with full position as payment"
        );
        require!(
            exit_amount > 0 && exit_amount <= payment.amount,
            "Invalid exit amount"
        );

        let exit_attributes: DualYieldTokenAttributes<Self::Api> =
            full_attributes.clone().into_part(&exit_amount);

        let lp_farm_exit_result = self.lp_farm_exit(
            orig_caller.clone(),
            full_attributes.lp_farm_token_nonce,
            full_attributes.lp_farm_token_amount,
            exit_attributes.lp_farm_token_amount,
        );
        let remove_liq_result = self.pair_remove_liquidity(
            lp_farm_exit_result.lp_tokens,
            pair_first_token_min_amount,
            pair_second_token_min_amount,
        );
        let staking_farm_exit_result = self.staking_farm_unstake(
            orig_caller,
            remove_liq_result.staking_token_payment,
            full_attributes.staking_farm_token_nonce,
            full_attributes.staking_farm_token_amount,
            exit_attributes.staking_farm_token_amount,
        );

        let opt_new_dual_yield_tokens = if exit_amount != total_for_nonce {
            let remaining_lp_farm_tokens = lp_farm_exit_result.remaining_farm_tokens.amount;
            let remaining_staking_farm_tokens =
                staking_farm_exit_result.remaining_farm_tokens.amount;
            let new_dual_yield_tokens = self.create_dual_yield_tokens(
                &dual_yield_token_mapper,
                full_attributes.lp_farm_token_nonce,
                remaining_lp_farm_tokens,
                full_attributes.staking_farm_token_nonce,
                remaining_staking_farm_tokens,
            );

            Some(new_dual_yield_tokens)
        } else {
            None
        };

        let caller = self.blockchain().get_caller();
        let unstake_result = UnstakeResult {
            other_token_payment: remove_liq_result.other_token_payment,
            lp_farm_rewards: lp_farm_exit_result.lp_farm_rewards,
            staking_rewards: staking_farm_exit_result.staking_rewards,
            unbond_staking_farm_token: staking_farm_exit_result.unbond_staking_farm_token,
            opt_new_dual_yield_tokens,
        };

        dual_yield_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        unstake_result.send_and_return(self, &caller)
    }
}
