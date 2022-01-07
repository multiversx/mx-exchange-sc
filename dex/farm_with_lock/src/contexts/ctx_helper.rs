elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::base::*;
use super::enter_farm::*;

#[elrond_wasm::module]
pub trait CtxHelper: config::ConfigModule + token_send::TokenSendModule {
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

    fn load_state(&self, context: &mut dyn Context<Self::Api>) {
        context.set_contract_state(self.state().get());
    }

    fn load_farm_token_id(&self, context: &mut dyn Context<Self::Api>) {
        context.set_farm_token_id(self.farm_token_id().get());
    }

    fn load_farming_token_id(&self, context: &mut dyn Context<Self::Api>) {
        context.set_farming_token_id(self.farming_token_id().get());
    }

    fn load_reward_token_id(&self, context: &mut dyn Context<Self::Api>) {
        context.set_reward_token_id(self.reward_token_id().get());
    }
}
