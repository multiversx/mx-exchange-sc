use crate::dual_yield_token::DualYieldTokenAttributes;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ProxyMergePosModule:
    crate::dual_yield_token::DualYieldTokenModule
    + crate::external_contracts_interactions::ExternalContractsInteractionsModule
    + crate::lp_farm_token::LpFarmTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
    + token_send::TokenSendModule
    + sc_whitelist_module::SCWhitelistModule
{
    #[payable("*")]
    #[endpoint(mergeMetastakingWithStakingToken)]
    fn merge_metastaking_with_staking_token(&self) -> EsdtTokenPayment {
        let mut payments = self.call_value().all_esdt_transfers();
        require!(
            payments.len() >= 2,
            "Must send metastaking token and at least a staking token"
        );

        let dual_yield_token = self.pop_first_payment(&mut payments);
        let dual_yield_token_mapper = self.dual_yield_token();
        dual_yield_token_mapper.require_same_token(&dual_yield_token.token_identifier);

        let staking_farm_token_id = self.staking_farm_token_id().get();
        let mut total_user_farm_staking_amount = BigUint::zero();
        for payment in &payments {
            require!(
                payment.token_identifier == staking_farm_token_id,
                "Invalid staking farm token"
            );

            total_user_farm_staking_amount += &payment.amount;

            self.send().esdt_local_burn(
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );
        }

        let mut attributes: DualYieldTokenAttributes<Self::Api> = self
            .get_attributes_as_part_of_fixed_supply(&dual_yield_token, &dual_yield_token_mapper);
        attributes.user_staking_farm_token_amount += total_user_farm_staking_amount;

        let caller = self.blockchain().get_caller();
        let new_dual_yield_tokens =
            self.create_dual_yield_tokens(&dual_yield_token_mapper, &attributes);
        self.send()
            .direct_non_zero_esdt_payment(&caller, &new_dual_yield_tokens);

        dual_yield_token_mapper.nft_burn(dual_yield_token.token_nonce, &dual_yield_token.amount);

        new_dual_yield_tokens
    }
}
