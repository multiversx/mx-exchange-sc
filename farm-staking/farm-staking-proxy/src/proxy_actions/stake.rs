use crate::{dual_yield_token::DualYieldTokenAttributes, result_types::StakeProxyResult};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ProxyStakeModule:
    crate::dual_yield_token::DualYieldTokenModule
    + crate::external_contracts_interactions::ExternalContractsInteractionsModule
    + crate::lp_farm_token::LpFarmTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
    + token_send::TokenSendModule
    + energy_query::EnergyQueryModule
    + sc_whitelist_module::SCWhitelistModule
{
    #[payable("*")]
    #[endpoint(stakeFarmTokens)]
    fn stake_farm_tokens(
        &self,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> StakeProxyResult<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);
        let payments = self.get_non_empty_payments();

        let output_payments = self.stake_farm_tokens_common(orig_caller, payments);

        output_payments.send_and_return(self, &caller)
    }

    fn stake_farm_tokens_common(
        &self,
        original_caller: ManagedAddress,
        payments: ManagedVec<EsdtTokenPayment>,
    ) -> StakeProxyResult<Self::Api> {
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
            original_caller.clone(),
            staking_token_amount,
            additional_staking_farm_tokens,
        );
        let received_staking_farm_token = staking_farm_enter_result.received_staking_farm_token;

        let (merged_lp_farm_tokens, lp_farm_boosted_rewards) = self
            .merge_lp_farm_tokens(
                original_caller,
                lp_farm_token_payment,
                additional_lp_farm_tokens,
            )
            .into_tuple();

        let new_attributes = DualYieldTokenAttributes {
            lp_farm_token_nonce: merged_lp_farm_tokens.token_nonce,
            lp_farm_token_amount: merged_lp_farm_tokens.amount,
            staking_farm_token_nonce: received_staking_farm_token.token_nonce,
            staking_farm_token_amount: received_staking_farm_token.amount,
        };
        let new_dual_yield_tokens =
            self.create_dual_yield_tokens(&dual_yield_token_mapper, &new_attributes);

        StakeProxyResult {
            dual_yield_tokens: new_dual_yield_tokens,
            staking_boosted_rewards: staking_farm_enter_result.boosted_rewards,
            lp_farm_boosted_rewards,
        }
    }
}
