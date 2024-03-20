use crate::{dual_yield_token::DualYieldTokenAttributes, result_types::ClaimDualYieldResult};

multiversx_sc::imports!();

pub struct InternalClaimResult<M: ManagedTypeApi> {
    pub lp_farm_rewards: EsdtTokenPayment<M>,
    pub staking_farm_rewards: EsdtTokenPayment<M>,
    pub new_dual_yield_attributes: DualYieldTokenAttributes<M>,
}

pub struct ClaimDualYieldArguments<'a, M: ManagedTypeApi> {
    pub caller: &'a ManagedAddress<M>,
    pub opt_orig_caller: OptionalValue<ManagedAddress<M>>,
    pub staking_claim_amount: BigUint<M>,
    pub attributes: DualYieldTokenAttributes<M>,
    pub get_rewards_unlocked: bool,
}

#[multiversx_sc::module]
pub trait ProxyClaimModule:
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
    #[endpoint(claimDualYield)]
    fn claim_dual_yield_endpoint(
        &self,
        get_rewards_unlocked: bool,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> ClaimDualYieldResult<Self::Api> {
        let payment = self.call_value().single_esdt();
        let dual_yield_token_mapper = self.dual_yield_token();
        dual_yield_token_mapper.require_same_token(&payment.token_identifier);

        let caller = self.blockchain().get_caller();
        let attributes: DualYieldTokenAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(&payment, &dual_yield_token_mapper);

        let claim_args = ClaimDualYieldArguments {
            caller: &caller,
            opt_orig_caller,
            staking_claim_amount: attributes.staking_farm_token_amount.clone(),
            attributes,
            get_rewards_unlocked,
        };
        let internal_claim_result = self.claim_dual_yield(claim_args);

        let new_dual_yield_tokens = self.create_dual_yield_tokens(
            &dual_yield_token_mapper,
            &internal_claim_result.new_dual_yield_attributes,
        );
        let claim_result = ClaimDualYieldResult {
            lp_farm_rewards: internal_claim_result.lp_farm_rewards,
            staking_farm_rewards: internal_claim_result.staking_farm_rewards,
            new_dual_yield_tokens,
        };

        dual_yield_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        claim_result.send_and_return(self, &caller)
    }

    fn claim_dual_yield(
        &self,
        args: ClaimDualYieldArguments<Self::Api>,
    ) -> InternalClaimResult<Self::Api> {
        let orig_caller = self.get_orig_caller_from_opt(args.caller, args.opt_orig_caller);

        let lp_tokens_in_position = self.get_lp_tokens_in_farm_position(
            args.attributes.lp_farm_token_nonce,
            &args.attributes.lp_farm_token_amount,
        );
        let new_staking_farm_value = self.get_lp_tokens_safe_price(lp_tokens_in_position);

        let staking_farm_token_id = self.staking_farm_token_id().get();
        let lp_farm_token_id = self.lp_farm_token_id().get();
        let lp_farm_payment = EsdtTokenPayment::new(
            lp_farm_token_id,
            args.attributes.lp_farm_token_nonce,
            args.attributes.lp_farm_token_amount,
        );
        let lp_farm_claim_rewards_result = self.lp_farm_claim_rewards(
            orig_caller.clone(),
            lp_farm_payment,
            args.get_rewards_unlocked,
        );
        let staking_farm_claim_rewards_result = self.staking_farm_claim_rewards(
            orig_caller,
            staking_farm_token_id,
            args.attributes.staking_farm_token_nonce,
            args.staking_claim_amount,
            new_staking_farm_value,
        );

        let new_lp_farm_tokens = lp_farm_claim_rewards_result.new_lp_farm_tokens;
        let new_staking_farm_tokens = staking_farm_claim_rewards_result.new_staking_farm_tokens;
        let new_attributes = DualYieldTokenAttributes {
            lp_farm_token_nonce: new_lp_farm_tokens.token_nonce,
            lp_farm_token_amount: new_lp_farm_tokens.amount,
            staking_farm_token_nonce: new_staking_farm_tokens.token_nonce,
            staking_farm_token_amount: new_staking_farm_tokens.amount,
        };

        InternalClaimResult {
            lp_farm_rewards: lp_farm_claim_rewards_result.lp_farm_rewards,
            staking_farm_rewards: staking_farm_claim_rewards_result.staking_farm_rewards,
            new_dual_yield_attributes: new_attributes,
        }
    }
}
