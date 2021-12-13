elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use itertools::Itertools;

use super::add_liquidity::*;
use super::base::*;

#[elrond_wasm::module]
pub trait CtxHelper: crate::config::ConfigModule + token_send::TokenSendModule {
    fn new_add_liquidity_context(
        &self,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> AddLiquidityContext<Self::Api> {
        let payment_tuple: Option<(EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>)> =
            self.get_all_payments_managed_vec()
                .into_iter()
                .collect_tuple();
        let (first_payment, second_payment) = match payment_tuple {
            Some(tuple) => (Some(tuple.0), Some(tuple.1)),
            None => (None, None),
        };

        let args = AddLiquidityArgs::new(
            first_token_amount_min,
            second_token_amount_min,
            opt_accept_funds_func,
        );
        let payments = AddLiquidityPayments::new(first_payment, second_payment);
        let tx_input = AddLiquidityTxInput::new(args, payments);

        AddLiquidityContext::new(tx_input)
    }

    fn read_state(&self, context: &mut dyn Context<Self::Api>) {
        context.set_contract_state(self.state().get());
    }

    fn read_lp_token_id(&self, context: &mut dyn Context<Self::Api>) {
        context.set_lp_token_id(self.lp_token_identifier().get());
    }

    fn read_first_token_id(&self, context: &mut dyn Context<Self::Api>) {
        context.set_first_token_id(self.first_token_id().get());
    }

    fn read_second_token_id(&self, context: &mut dyn Context<Self::Api>) {
        context.set_second_token_id(self.second_token_id().get());
    }
}
