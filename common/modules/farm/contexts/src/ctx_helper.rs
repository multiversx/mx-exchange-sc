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
{
    fn new_farm_context(
        &self,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> GenericContext<Self::Api> {
        let caller = self.blockchain().get_caller();

        let payments = self.call_value().all_esdt_transfers();
        let mut payments_iter = payments.iter();

        let first_payment = payments_iter.next().unwrap();

        let mut additional_payments = ManagedVec::new();
        for payment in payments_iter {
            additional_payments.push(payment);
        }

        let args = GenericArgs::new(opt_accept_funds_func);
        let payments = GenericPayments::new(first_payment, additional_payments);
        let tx = GenericTxInput::new(args, payments);

        GenericContext::new(tx, caller)
    }

    #[inline]
    fn load_state(&self, context: &mut GenericContext<Self::Api>) {
        context.set_contract_state(self.state().get());
    }

    #[inline]
    fn load_farm_token_id(&self, context: &mut GenericContext<Self::Api>) {
        context.set_farm_token_id(self.farm_token_id().get());
    }

    #[inline]
    fn load_farming_token_id(&self, context: &mut GenericContext<Self::Api>) {
        context.set_farming_token_id(self.farming_token_id().get());
    }

    #[inline]
    fn load_reward_token_id(&self, context: &mut GenericContext<Self::Api>) {
        context.set_reward_token_id(self.reward_token_id().get());
    }

    #[inline]
    fn load_block_nonce(&self, context: &mut GenericContext<Self::Api>) {
        context.set_block_nonce(self.blockchain().get_block_nonce());
    }

    #[inline]
    fn load_block_epoch(&self, context: &mut GenericContext<Self::Api>) {
        context.set_block_epoch(self.blockchain().get_block_epoch());
    }

    #[inline]
    fn load_reward_reserve(&self, context: &mut GenericContext<Self::Api>) {
        context.set_reward_reserve(self.reward_reserve().get());
    }

    #[inline]
    fn load_reward_per_share(&self, context: &mut GenericContext<Self::Api>) {
        context.set_reward_per_share(self.reward_per_share().get());
    }

    #[inline]
    fn load_farm_token_supply(&self, context: &mut GenericContext<Self::Api>) {
        context.set_farm_token_supply(self.farm_token_supply().get());
    }

    #[inline]
    fn load_division_safety_constant(&self, context: &mut GenericContext<Self::Api>) {
        context.set_division_safety_constant(self.division_safety_constant().get());
    }

    #[inline]
    fn commit_changes(&self, context: &GenericContext<Self::Api>) {
        if let Some(value) = context.get_reward_reserve() {
            self.reward_reserve().set(value);
        }
        if let Some(value) = context.get_reward_per_share() {
            self.reward_per_share().set(value);
        }
    }

    #[inline]
    fn execute_output_payments(&self, context: &GenericContext<Self::Api>) {
        self.send_multiple_tokens_if_not_zero(
            context.get_caller(),
            context.get_output_payments(),
            context.get_opt_accept_funds_func(),
        );
    }

    #[inline]
    fn load_farm_attributes(&self, context: &mut GenericContext<Self::Api>) {
        let farm_token_id = context.get_farm_token_id().unwrap().clone();
        let nonce = context
            .get_tx_input()
            .get_payments()
            .get_first()
            .token_nonce;

        context.set_input_attributes(
            self.blockchain()
                .get_esdt_token_data(&self.blockchain().get_sc_address(), &farm_token_id, nonce)
                .decode_attributes_or_exit(),
        )
    }

    #[inline]
    fn calculate_reward(&self, context: &mut GenericContext<Self::Api>) {
        let reward = if context.get_reward_per_share().unwrap()
            > &context.get_input_attributes().unwrap().reward_per_share
        {
            &context.get_tx_input().get_payments().get_first().amount
                * &(context.get_reward_per_share().unwrap()
                    - &context.get_input_attributes().unwrap().reward_per_share)
                / context.get_division_safety_constant().unwrap()
        } else {
            BigUint::zero()
        };

        context.set_position_reward(reward);
    }

    fn calculate_initial_farming_amount(&self, context: &mut GenericContext<Self::Api>) {
        let initial_farming_token_amount = self.rule_of_three_non_zero_result(
            &context.get_tx_input().get_payments().get_first().amount,
            &context.get_input_attributes().unwrap().current_farm_amount,
            &context
                .get_input_attributes()
                .unwrap()
                .initial_farming_amount,
        );

        context.set_initial_farming_amount(initial_farming_token_amount);
    }

    fn increase_reward_with_compounded_rewards(&self, context: &mut GenericContext<Self::Api>) {
        let amount = self.rule_of_three(
            &context.get_tx_input().get_payments().get_first().amount,
            &context.get_input_attributes().unwrap().current_farm_amount,
            &context.get_input_attributes().unwrap().compounded_reward,
        );

        context.increase_position_reward(&amount);
    }

    fn construct_output_payments_exit(&self, context: &mut GenericContext<Self::Api>) {
        let mut result = ManagedVec::new();

        result.push(self.create_payment(
            context.get_farming_token_id().unwrap(),
            0,
            context.get_initial_farming_amount().unwrap(),
        ));

        context.set_output_payments(result);
    }

    fn construct_and_get_result(
        &self,
        context: &GenericContext<Self::Api>,
    ) -> MultiResult2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> {
        MultiResult2::from((
            context.get_output_payments().get(0),
            context.get_final_reward().unwrap().clone(),
        ))
    }
}
