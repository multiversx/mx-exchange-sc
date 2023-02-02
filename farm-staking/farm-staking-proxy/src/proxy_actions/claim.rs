use crate::{dual_yield_token::DualYieldTokenAttributes, result_types::ClaimDualYieldResult};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ProxyClaimModule:
    crate::dual_yield_token::DualYieldTokenModule
    + crate::external_contracts_interactions::ExternalContractsInteractionsModule
    + crate::lp_farm_token::LpFarmTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
    + token_send::TokenSendModule
    + sc_whitelist_module::SCWhitelistModule
{
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
        let lp_tokens_safe_price = self.get_lp_tokens_safe_price(lp_tokens_in_position);
        let new_staking_farm_value =
            &lp_tokens_safe_price + &attributes.user_staking_farm_token_amount;

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
        let new_attributes = DualYieldTokenAttributes {
            lp_farm_token_nonce: new_lp_farm_tokens.token_nonce,
            lp_farm_token_amount: new_lp_farm_tokens.amount,
            staking_farm_token_nonce: new_staking_farm_tokens.token_nonce,
            staking_farm_token_amount: lp_tokens_safe_price,
            user_staking_farm_token_amount: attributes.user_staking_farm_token_amount,
        };
        let new_dual_yield_tokens =
            self.create_dual_yield_tokens(&dual_yield_token_mapper, &new_attributes);

        dual_yield_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        let caller = self.blockchain().get_caller();
        let claim_result = ClaimDualYieldResult {
            lp_farm_rewards: lp_farm_claim_rewards_result.lp_farm_rewards,
            staking_farm_rewards: staking_farm_claim_rewards_result.staking_farm_rewards,
            new_dual_yield_tokens,
        };

        claim_result.send_and_return(self, &caller)
    }
}
