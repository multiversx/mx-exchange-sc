elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::base::*;
use super::enter_farm::*;
use super::exit_farm::*;
use crate::assert;
use crate::errors::*;

#[elrond_wasm::module]
pub trait CtxHelper:
    config::ConfigModule
    + token_send::TokenSendModule
    + rewards::RewardsModule
    + farm_token::FarmTokenModule
{
    fn new_enter_farm_context(
        &self,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> EnterFarmContext<Self::Api> {
        let caller = self.blockchain().get_caller();

        let payments = self.call_value().all_esdt_transfers();
        let mut payments_iter = payments.iter();

        let first_payment = payments_iter.next().unwrap();

        let mut additional_payments = ManagedVec::new();
        while let Some(payment) = payments_iter.next() {
            additional_payments.push(payment);
        }

        let args = EnterFarmArgs::new(opt_accept_funds_func);
        let payments = EnterFarmPayments::new(first_payment, additional_payments);
        let tx = EnterFarmTxInput::new(args, payments);

        EnterFarmContext::new(tx, caller)
    }

    fn new_exit_farm_context(
        &self,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> ExitFarmContext<Self::Api> {
        let caller = self.blockchain().get_caller();

        let payments = self.call_value().all_esdt_transfers();
        let mut payments_iter = payments.iter();

        let first_payment = payments_iter.next().unwrap();
        assert!(self, payments_iter.next().is_none(), ERROR_BAD_PAYMENTS_LEN);

        let args = ExitFarmArgs::new(opt_accept_funds_func);
        let payments = ExitFarmPayments::new(first_payment);
        let tx = ExitFarmTxInput::new(args, payments);

        ExitFarmContext::new(tx, caller)
    }

    #[inline]
    fn load_state(&self, context: &mut dyn Context<Self::Api>) {
        context.set_contract_state(self.state().get());
    }

    #[inline]
    fn load_farm_token_id(&self, context: &mut dyn Context<Self::Api>) {
        context.set_farm_token_id(self.farm_token_id().get());
    }

    #[inline]
    fn load_farming_token_id(&self, context: &mut dyn Context<Self::Api>) {
        context.set_farming_token_id(self.farming_token_id().get());
    }

    #[inline]
    fn load_reward_token_id(&self, context: &mut dyn Context<Self::Api>) {
        context.set_reward_token_id(self.reward_token_id().get());
    }

    #[inline]
    fn load_block_nonce(&self, context: &mut dyn Context<Self::Api>) {
        context.set_reward_token_id(self.reward_token_id().get());
    }

    #[inline]
    fn load_block_epoch(&self, context: &mut dyn Context<Self::Api>) {
        context.set_reward_token_id(self.reward_token_id().get());
    }

    #[inline]
    fn load_reward_reserve(&self, context: &mut dyn Context<Self::Api>) {
        context.set_reward_reserve(self.reward_reserve().get());
    }

    #[inline]
    fn load_reward_per_share(&self, context: &mut dyn Context<Self::Api>) {
        context.set_reward_per_share(self.reward_per_share().get());
    }

    #[inline]
    fn load_farm_token_supply(&self, context: &mut dyn Context<Self::Api>) {
        context.set_farm_token_supply(self.farm_token_supply().get());
    }

    #[inline]
    fn load_division_safety_constant(&self, context: &mut dyn Context<Self::Api>) {
        context.set_division_safety_constant(self.division_safety_constant().get());
    }

    #[inline]
    fn commit_changes(&self, context: &dyn Context<Self::Api>) {
        self.reward_reserve().set(context.get_reward_per_share());
        self.reward_per_share().set(context.get_reward_per_share());
        self.farm_token_supply()
            .set(context.get_farm_token_supply());
    }

    #[inline]
    fn execute_output_payments(&self, context: &dyn Context<Self::Api>) {
        let result = self.send_multiple_tokens_if_not_zero(
            context.get_caller(),
            context.get_output_payments(),
            context.get_opt_accept_funds_func(),
        );
        assert!(self, result.is_ok(), ERROR_PAYMENT_FAILED);
    }

    #[inline]
    fn load_farm_attributes(&self, context: &mut ExitFarmContext<Self::Api>) {
        let farm_token_id = context.get_farm_token_id().clone();
        let nonce = context
            .get_tx_input()
            .get_payments()
            .get_first()
            .token_nonce;

        context.set_input_attributes(
            self.blockchain()
                .get_esdt_token_data(&self.blockchain().get_sc_address(), &farm_token_id, nonce)
                .decode_attributes()
                .unwrap(),
        )
    }

    #[inline]
    fn calculate_reward(&self, context: &mut ExitFarmContext<Self::Api>) {
        let reward = if context.get_reward_per_share()
            > &context
                .get_input_attributes()
                .unwrap()
                .initial_farming_amount
        {
            context.get_tx_input().get_payments().get_first().amount
                * &(context.get_reward_per_share()
                    - &context
                        .get_input_attributes()
                        .unwrap()
                        .initial_farming_amount)
                / context.get_division_safety_constant()
        } else {
            BigUint::zero()
        };

        context.set_position_reward(reward);
    }
}
