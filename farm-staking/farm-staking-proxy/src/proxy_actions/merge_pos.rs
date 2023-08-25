use common_structs::PaymentsVec;

use crate::{dual_yield_token::DualYieldTokenAttributes, result_types::MergeResult};

use mergeable::Mergeable;
use unwrappable::Unwrappable;

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
        let mut payments = self.call_value().all_esdt_transfers().clone_value();
        require!(
            payments.len() >= 2,
            "Must send metastaking token and at least a staking token"
        );

        let dual_yield_token = self.pop_first_payment(&mut payments);
        let dual_yield_token_mapper = self.dual_yield_token();
        dual_yield_token_mapper.require_same_token(&dual_yield_token.token_identifier);

        let mut attributes: DualYieldTokenAttributes<Self::Api> = self
            .get_attributes_as_part_of_fixed_supply(&dual_yield_token, &dual_yield_token_mapper);
        dual_yield_token_mapper.nft_burn(dual_yield_token.token_nonce, &dual_yield_token.amount);

        let caller = self.blockchain().get_caller();
        let staking_farm_rewards = self.claim_staking_rewards_before_merge(&caller, &payments);

        let staking_amount_before_merge = attributes.get_total_staking_token_amount();
        for farm_staking_token in &payments {
            attributes.real_pos_token_amount += &farm_staking_token.amount;
        }

        let mut dual_yield_claim_result = self.claim_dual_yield(
            &caller,
            OptionalValue::None,
            staking_amount_before_merge,
            attributes,
        );
        dual_yield_claim_result
            .staking_farm_rewards
            .merge_with(staking_farm_rewards);

        let new_dual_yield_tokens = self.create_dual_yield_tokens(
            &dual_yield_token_mapper,
            &dual_yield_claim_result.new_dual_yield_attributes,
        );
        let merge_result = MergeResult {
            lp_farm_rewards: dual_yield_claim_result.lp_farm_rewards,
            staking_farm_rewards: dual_yield_claim_result.staking_farm_rewards,
            new_dual_yield_tokens,
        };

        merge_result.send_and_return(self, &caller)
    }

    fn claim_staking_rewards_before_merge(
        &self,
        caller: &ManagedAddress,
        farm_staking_tokens: &PaymentsVec<Self::Api>,
    ) -> EsdtTokenPayment {
        let staking_farm_token_id = self.staking_farm_token_id().get();
        let mut opt_staking_farm_rewards = Option::<EsdtTokenPayment>::None;
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

            match &mut opt_staking_farm_rewards {
                Some(rew) => rew.merge_with(staking_claim_result.staking_farm_rewards),
                None => {
                    opt_staking_farm_rewards = Some(staking_claim_result.staking_farm_rewards);
                }
            };

            let new_staking_farm_tokens = staking_claim_result.new_staking_farm_tokens;
            self.send().esdt_local_burn(
                &new_staking_farm_tokens.token_identifier,
                new_staking_farm_tokens.token_nonce,
                &new_staking_farm_tokens.amount,
            );
        }

        opt_staking_farm_rewards.unwrap_or_panic::<Self::Api>()
    }
}
