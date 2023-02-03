use common_structs::PaymentsVec;

use crate::{dual_yield_token::DualYieldTokenAttributes, result_types::MergeResult};

use super::claim::InternalClaimResult;

use mergeable::Mergeable;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait ProxyMergePosModule:
    crate::dual_yield_token::DualYieldTokenModule
    + crate::external_contracts_interactions::ExternalContractsInteractionsModule
    + crate::lp_farm_token::LpFarmTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
    + token_send::TokenSendModule
    + sc_whitelist_module::SCWhitelistModule
    + super::claim::ProxyClaimModule
{
    #[payable("*")]
    #[endpoint(mergeMetastakingWithStakingToken)]
    fn merge_metastaking_with_staking_token(&self) -> MergeResult<Self::Api> {
        let mut payments = self.call_value().all_esdt_transfers();
        require!(
            payments.len() >= 2,
            "Must send metastaking token and at least a staking token"
        );

        let dual_yield_token = self.pop_first_payment(&mut payments);
        let dual_yield_token_mapper = self.dual_yield_token();
        dual_yield_token_mapper.require_same_token(&dual_yield_token.token_identifier);

        let caller = self.blockchain().get_caller();
        let claim_result = self.claim_rewards_before_merge(&caller, dual_yield_token, &payments);
        let new_dual_yield_tokens = self.merge_into_single_metastaking_token(
            &dual_yield_token_mapper,
            claim_result.new_dual_yield_attributes,
            payments,
        );

        let merge_result = MergeResult {
            lp_farm_rewards: claim_result.lp_farm_rewards,
            staking_farm_rewards: claim_result.staking_farm_rewards,
            new_dual_yield_tokens,
        };
        merge_result.send_and_return(self, &caller)
    }

    fn claim_rewards_before_merge(
        &self,
        caller: &ManagedAddress,
        dual_yield_token: EsdtTokenPayment,
        farm_staking_tokens: &PaymentsVec<Self::Api>,
    ) -> InternalClaimResult<Self::Api> {
        let staking_farm_token_id = self.staking_farm_token_id().get();
        let mut claim_result = self.claim_dual_yield(caller, OptionalValue::None, dual_yield_token);
        for farm_staking_token in farm_staking_tokens {
            require!(
                farm_staking_token.token_identifier == staking_farm_token_id,
                "Invalid staking farm token"
            );

            let staking_claim_result = self.staking_farm_claim_rewards(
                caller.clone(),
                farm_staking_token.token_identifier,
                farm_staking_token.token_nonce,
                farm_staking_token.amount.clone(),
                farm_staking_token.amount,
            );

            claim_result
                .staking_farm_rewards
                .merge_with(staking_claim_result.staking_farm_rewards);

            let new_staking_farm_tokens = staking_claim_result.new_staking_farm_tokens;
            self.send().esdt_local_burn(
                &new_staking_farm_tokens.token_identifier,
                new_staking_farm_tokens.token_nonce,
                &new_staking_farm_tokens.amount,
            );
        }

        claim_result
    }

    fn merge_into_single_metastaking_token(
        &self,
        dual_yield_token_mapper: &NonFungibleTokenMapper,
        mut attributes: DualYieldTokenAttributes<Self::Api>,
        farm_staking_tokens: PaymentsVec<Self::Api>,
    ) -> EsdtTokenPayment {
        for farm_staking_token in &farm_staking_tokens {
            attributes.user_staking_farm_token_amount += &farm_staking_token.amount;
        }

        self.create_dual_yield_tokens(dual_yield_token_mapper, &attributes)
    }
}
