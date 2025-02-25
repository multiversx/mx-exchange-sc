multiversx_sc::imports!();

use common_structs::FarmTokenAttributes;
use farm_staking::token_attributes::StakingFarmTokenAttributes;

use crate::{
    dual_yield_token::DualYieldTokenAttributes,
    result_types::{ClaimDualYieldResult, StakeProxyResult},
};

#[multiversx_sc::module]
pub trait ProxyExternalInteractionsModule:
    crate::dual_yield_token::DualYieldTokenModule
    + crate::external_contracts_interactions::ExternalContractsInteractionsModule
    + crate::lp_farm_token::LpFarmTokenModule
    + crate::proxy_actions::stake::ProxyStakeModule
    + crate::proxy_actions::claim::ProxyClaimModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + permissions_hub_module::PermissionsHubModule
    + utils::UtilsModule
    + token_send::TokenSendModule
    + energy_query::EnergyQueryModule
    + sc_whitelist_module::SCWhitelistModule
{
    #[payable("*")]
    #[endpoint(stakeFarmOnBehalf)]
    fn stake_farm_on_behalf(&self, original_owner: ManagedAddress) -> StakeProxyResult<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_user_whitelisted(&original_owner, &caller);

        let payments = self.get_non_empty_payments();
        self.check_stake_farm_payments(&original_owner, &payments);

        let output_payments = self.stake_farm_tokens_common(original_owner.clone(), payments);

        self.send_payment_non_zero(&original_owner, &output_payments.lp_farm_boosted_rewards);
        self.send_payment_non_zero(&original_owner, &output_payments.staking_boosted_rewards);
        self.send_payment_non_zero(&caller, &output_payments.dual_yield_tokens);

        output_payments
    }

    #[payable("*")]
    #[endpoint(claimDualYieldOnBehalf)]
    fn claim_dual_yield_on_behalf(&self) -> ClaimDualYieldResult<Self::Api> {
        let payment = self.call_value().single_esdt();

        let caller = self.blockchain().get_caller();
        let original_owner = self.get_underlying_positions_original_owner(&payment);
        self.require_user_whitelisted(&original_owner, &caller);

        let claim_result = self.claim_dual_yield_common(original_owner.clone(), payment);

        self.send_payment_non_zero(&original_owner, &claim_result.lp_farm_rewards);
        self.send_payment_non_zero(&original_owner, &claim_result.staking_farm_rewards);
        self.send_payment_non_zero(&caller, &claim_result.new_dual_yield_tokens);

        claim_result
    }

    fn check_stake_farm_payments(
        &self,
        original_owner: &ManagedAddress,
        payments: &ManagedVec<EsdtTokenPayment>,
    ) {
        let lp_farm_token_payment = payments.get(0);
        let additional_payments = payments.slice(1, payments.len()).unwrap_or_default();

        let lp_farm_token_id = self.lp_farm_token_id().get();
        require!(
            lp_farm_token_payment.token_identifier == lp_farm_token_id,
            "Invalid first payment"
        );

        let attributes = self
            .blockchain()
            .get_token_attributes::<FarmTokenAttributes<Self::Api>>(
                &lp_farm_token_payment.token_identifier,
                lp_farm_token_payment.token_nonce,
            );

        require!(
            &attributes.original_owner == original_owner,
            "Provided address is not the same as the original owner"
        );

        for payment in additional_payments.into_iter() {
            require!(
                &self.get_underlying_positions_original_owner(&payment) == original_owner,
                "Provided address is not the same as the original owner"
            );
        }
    }

    fn get_underlying_positions_original_owner(
        &self,
        payment: &EsdtTokenPayment,
    ) -> ManagedAddress {
        let dual_yield_token_mapper = self.dual_yield_token();
        dual_yield_token_mapper.require_same_token(&payment.token_identifier);

        let dual_yield_attributes: DualYieldTokenAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(payment, &dual_yield_token_mapper);

        let lp_farm_token_id = self.lp_farm_token_id().get();
        let lp_attributes = self
            .blockchain()
            .get_token_attributes::<FarmTokenAttributes<Self::Api>>(
                &lp_farm_token_id,
                dual_yield_attributes.lp_farm_token_nonce,
            );

        let lp_original_owner = lp_attributes.original_owner;
        require!(
            !lp_original_owner.is_zero(),
            "LP Token original owner incorrect"
        );

        let staking_farm_token_id = self.staking_farm_token_id().get();
        let staking_attributes = self
            .blockchain()
            .get_token_attributes::<StakingFarmTokenAttributes<Self::Api>>(
                &staking_farm_token_id,
                dual_yield_attributes.staking_farm_token_nonce,
            );

        let staking_original_owner = staking_attributes.original_owner;
        require!(
            lp_original_owner == staking_original_owner,
            "Underlying positions original owners do not match"
        );

        lp_original_owner
    }
}
