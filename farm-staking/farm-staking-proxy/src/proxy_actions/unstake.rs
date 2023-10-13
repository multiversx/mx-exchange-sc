use fixed_supply_token::FixedSupplyToken;

use crate::{dual_yield_token::DualYieldTokenAttributes, result_types::UnstakeResult};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ProxyUnstakeModule:
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
    #[endpoint(unstakeFarmTokens)]
    fn unstake_farm_tokens(
        &self,
        pair_first_token_min_amount: BigUint,
        pair_second_token_min_amount: BigUint,
        opt_orig_caller: OptionalValue<ManagedAddress>,
    ) -> UnstakeResult<Self::Api> {
        let caller = self.blockchain().get_caller();
        let orig_caller = self.get_orig_caller_from_opt(&caller, opt_orig_caller);
        let payment = self.call_value().single_esdt();
        let dual_yield_token_mapper = self.dual_yield_token();
        dual_yield_token_mapper.require_same_token(&payment.token_identifier);

        let full_attributes: DualYieldTokenAttributes<Self::Api> =
            dual_yield_token_mapper.get_token_attributes(payment.token_nonce);

        let exit_attributes: DualYieldTokenAttributes<Self::Api> =
            full_attributes.into_part(&payment.amount);

        let lp_farm_exit_result = self.lp_farm_exit(
            orig_caller.clone(),
            exit_attributes.lp_farm_token_nonce,
            exit_attributes.lp_farm_token_amount,
        );
        let remove_liq_result = self.pair_remove_liquidity(
            lp_farm_exit_result.lp_tokens,
            pair_first_token_min_amount,
            pair_second_token_min_amount,
        );

        let staking_farm_exit_result = self.staking_farm_unstake(
            orig_caller.clone(),
            remove_liq_result.staking_token_payment,
            exit_attributes.staking_farm_token_nonce,
            exit_attributes.staking_farm_token_amount,
        );

        let caller = self.blockchain().get_caller();
        let unstake_result = UnstakeResult {
            other_token_payment: remove_liq_result.other_token_payment,
            lp_farm_rewards: lp_farm_exit_result.lp_farm_rewards,
            staking_rewards: staking_farm_exit_result.staking_rewards,
            unbond_staking_farm_token: staking_farm_exit_result.unbond_staking_farm_token,
        };

        dual_yield_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        unstake_result.send_and_return(self, &caller)
    }
}
