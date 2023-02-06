use fixed_supply_token::FixedSupplyToken;
use mergeable::Mergeable;

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
    + sc_whitelist_module::SCWhitelistModule
{
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

        let full_staking_token_amount = full_attributes.get_total_staking_token_amount();
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

        let remaining_total_staking_farm_tokens =
            &full_staking_token_amount - &exit_attributes.virtual_pos_token_amount;
        let staking_farm_exit_result = self.staking_farm_unstake(
            orig_caller.clone(),
            remove_liq_result.staking_token_payment,
            full_attributes.virtual_pos_token_nonce,
            full_staking_token_amount,
            exit_attributes.virtual_pos_token_amount.clone(),
        );

        let opt_unstake_user_pos_result = if exit_attributes.real_pos_token_amount > 0 {
            let res = self.staking_farm_unstake_user_position(
                orig_caller,
                full_attributes.virtual_pos_token_nonce,
                remaining_total_staking_farm_tokens,
                exit_attributes.real_pos_token_amount.clone(),
            );
            Some(res)
        } else {
            None
        };

        let opt_new_dual_yield_tokens = if exit_amount != total_for_nonce {
            let remaining_lp_farm_tokens = lp_farm_exit_result.remaining_farm_tokens.amount;
            let remaining_virtual_farm_tokens =
                full_attributes.virtual_pos_token_amount - exit_attributes.virtual_pos_token_amount;
            let remaining_real_farm_tokens =
                full_attributes.real_pos_token_amount - exit_attributes.real_pos_token_amount;
            let new_attributes = DualYieldTokenAttributes {
                lp_farm_token_nonce: full_attributes.lp_farm_token_nonce,
                lp_farm_token_amount: remaining_lp_farm_tokens,
                virtual_pos_token_nonce: full_attributes.virtual_pos_token_nonce,
                virtual_pos_token_amount: remaining_virtual_farm_tokens,
                real_pos_token_amount: remaining_real_farm_tokens,
            };
            let new_dual_yield_tokens =
                self.create_dual_yield_tokens(&dual_yield_token_mapper, &new_attributes);

            Some(new_dual_yield_tokens)
        } else {
            None
        };

        let mut total_staking_rewards = staking_farm_exit_result.staking_rewards;
        let opt_unbond_token = opt_unstake_user_pos_result.map(|res| {
            total_staking_rewards.merge_with(res.staking_rewards);

            res.unbond_staking_farm_token
        });

        let caller = self.blockchain().get_caller();
        let unstake_result = UnstakeResult {
            other_token_payment: remove_liq_result.other_token_payment,
            lp_farm_rewards: lp_farm_exit_result.lp_farm_rewards,
            staking_rewards: total_staking_rewards,
            unbond_staking_farm_token: staking_farm_exit_result.unbond_staking_farm_token,
            opt_unbond_staking_farm_token_for_user_pos: opt_unbond_token,
            opt_new_dual_yield_tokens,
        };

        dual_yield_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        unstake_result.send_and_return(self, &caller)
    }
}
