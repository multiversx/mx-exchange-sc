elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::generic::*;

#[elrond_wasm::module]
pub trait CtxHelper:
    config::ConfigModule
    + token_send::TokenSendModule
    + rewards::RewardsModule
    + farm_token::FarmTokenModule
    + token_merge::TokenMergeModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn new_farm_context(&self) -> GenericContext<Self::Api> {
        GenericContext::new(self)
    }

    fn calculate_reward(&self, context: &mut GenericContext<Self::Api>) {
        let farm_token_attributes = context.get_input_attributes();
        let rps = context.get_reward_per_share();

        let reward = if rps > &farm_token_attributes.reward_per_share {
            let first_payment = &context.get_tx_input().first_payment;
            let rps_diff = rps - &farm_token_attributes.reward_per_share;
            let div_safety = context.get_division_safety_constant();

            &first_payment.amount * &rps_diff / div_safety
        } else {
            BigUint::zero()
        };

        context.set_position_reward(reward);
    }

    fn calculate_initial_farming_amount(&self, context: &mut GenericContext<Self::Api>) {
        let first_payment = &context.get_tx_input().first_payment;
        let farm_token_attributes = context.get_input_attributes();

        let initial_farming_token_amount = self.rule_of_three_non_zero_result(
            &first_payment.amount,
            &farm_token_attributes.current_farm_amount,
            &farm_token_attributes.initial_farming_amount,
        );

        context.set_initial_farming_amount(initial_farming_token_amount);
    }

    fn increase_reward_with_compounded_rewards(&self, context: &mut GenericContext<Self::Api>) {
        let first_payment = &context.get_tx_input().first_payment;
        let farm_token_attributes = context.get_input_attributes();

        let amount = self.rule_of_three(
            &first_payment.amount,
            &farm_token_attributes.current_farm_amount,
            &farm_token_attributes.compounded_reward,
        );

        context.increase_position_reward(&amount);
    }

    fn construct_output_payments_exit(&self, context: &mut GenericContext<Self::Api>) {
        let mut result = ManagedVec::new();

        result.push(EsdtTokenPayment::new(
            context.get_farming_token_id().clone(),
            0,
            context.get_initial_farming_amount().clone(),
        ));

        context.set_output_payments(result);
    }

    fn construct_and_get_result(
        &self,
        context: &GenericContext<Self::Api>,
    ) -> MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> {
        MultiValue2::from((
            context.get_output_payments().get(0),
            context.get_final_reward().unwrap().clone(),
        ))
    }

    fn commit_changes(&self, context: &GenericContext<Self::Api>) {
        self.reward_reserve().set(context.get_reward_reserve());
        self.reward_per_share().set(context.get_reward_per_share());
    }

    fn execute_output_payments(&self, context: &GenericContext<Self::Api>) {
        self.send_multiple_tokens_if_not_zero(context.get_caller(), context.get_output_payments());
    }
}
