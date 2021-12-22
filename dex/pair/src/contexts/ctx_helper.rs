elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use itertools::Itertools;

use crate::AddLiquidityResultType;
use crate::RemoveLiquidityResultType;

use super::add_liquidity::*;
use super::base::*;
use super::remove_liquidity::*;
use super::swap::SwapArgs;
use super::swap::SwapContext;
use super::swap::SwapPayments;
use super::swap::SwapTxInput;

#[elrond_wasm::module]
pub trait CtxHelper:
    crate::config::ConfigModule + token_send::TokenSendModule + crate::amm::AmmModule
{
    fn new_add_liquidity_context(
        &self,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> AddLiquidityContext<Self::Api> {
        let caller = self.blockchain().get_caller();

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

        AddLiquidityContext::new(tx_input, caller)
    }

    fn new_remove_liquidity_context(
        &self,
        payment_token: &TokenIdentifier,
        payment_nonce: u64,
        payment_amount: &BigUint,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> RemoveLiquidityContext<Self::Api> {
        let caller = self.blockchain().get_caller();

        let payment = self.create_payment(payment_token, payment_nonce, payment_amount);
        let args = RemoveLiquidityArgs::new(
            first_token_amount_min,
            second_token_amount_min,
            opt_accept_funds_func,
        );
        let payments = RemoveLiquidityPayments::new(payment);
        let tx_input = RemoveLiquidityTxInput::new(args, payments);

        RemoveLiquidityContext::new(tx_input, caller)
    }

    fn new_swap_context(
        &self,
        payment_token: &TokenIdentifier,
        payment_nonce: u64,
        payment_amount: &BigUint,
        out_token_id: TokenIdentifier,
        out_amount: BigUint,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SwapContext<Self::Api> {
        let caller = self.blockchain().get_caller();

        let payment = self.create_payment(payment_token, payment_nonce, payment_amount);
        let args = SwapArgs::new(out_token_id, out_amount, opt_accept_funds_func);
        let payments = SwapPayments::new(payment);
        let tx_input = SwapTxInput::new(args, payments);

        SwapContext::new(tx_input, caller)
    }

    fn load_state(&self, context: &mut dyn Context<Self::Api>) {
        context.set_contract_state(self.state().get());
    }

    fn load_lp_token_id(&self, context: &mut dyn Context<Self::Api>) {
        context.set_lp_token_id(self.lp_token_identifier().get());
    }

    fn load_pool_token_ids(&self, context: &mut dyn Context<Self::Api>) {
        context.set_first_token_id(self.first_token_id().get());
        context.set_second_token_id(self.second_token_id().get());
    }

    fn load_pool_reserves(&self, context: &mut dyn Context<Self::Api>) {
        let second_token_id = context.get_second_token_id().clone();
        let first_token_id = context.get_first_token_id().clone();
        context.set_first_token_reserve(self.pair_reserve(&first_token_id).get());
        context.set_second_token_reserve(self.pair_reserve(&second_token_id).get());
    }

    fn load_lp_token_supply(&self, context: &mut dyn Context<Self::Api>) {
        context.set_lp_token_supply(self.lp_token_supply().get());
    }

    fn load_initial_k(&self, context: &mut dyn Context<Self::Api>) {
        let k = self.calculate_k_constant(
            context.get_first_token_reserve(),
            context.get_second_token_reserve(),
        );
        context.set_initial_k(k);
    }

    fn construct_add_liquidity_output_payments(
        &self,
        context: &mut AddLiquidityContext<Self::Api>,
    ) {
        let mut payments: ManagedVec<EsdtTokenPayment<Self::Api>> = ManagedVec::new();

        payments.push(self.create_payment(
            context.get_lp_token_id(),
            0,
            context.get_liquidity_added(),
        ));
        payments.push(self.create_payment(
            context.get_first_token_id(),
            0,
            &(&context.get_first_payment().amount - context.get_first_amount_optimal()),
        ));
        payments.push(self.create_payment(
            context.get_second_token_id(),
            0,
            &(&context.get_second_payment().amount - context.get_second_amount_optimal()),
        ));

        context.set_output_payments(payments);
    }

    fn construct_remove_liquidity_output_payments(
        &self,
        context: &mut RemoveLiquidityContext<Self::Api>,
    ) {
        let mut payments: ManagedVec<EsdtTokenPayment<Self::Api>> = ManagedVec::new();

        payments.push(self.create_payment(
            context.get_first_token_id(),
            0,
            context.get_first_token_amount_removed(),
        ));
        payments.push(self.create_payment(
            context.get_second_token_id(),
            0,
            context.get_second_token_amount_removed(),
        ));

        context.set_output_payments(payments);
    }

    fn execute_output_payments(&self, context: &dyn Context<Self::Api>) {
        self.send_multiple_tokens_if_not_zero(
            context.get_caller(),
            context.get_output_payments(),
            context.get_opt_accept_funds_func(),
        )
        .unwrap();
    }

    fn commit_changes(&self, context: &dyn Context<Self::Api>) {
        self.pair_reserve(context.get_first_token_id())
            .set(context.get_first_token_reserve());
        self.pair_reserve(context.get_second_token_id())
            .set(context.get_second_token_reserve());
        self.lp_token_supply().set(context.get_lp_token_supply());
    }

    fn construct_and_get_add_liquidity_output_results(
        &self,
        context: &AddLiquidityContext<Self::Api>,
    ) -> AddLiquidityResultType<Self::Api> {
        MultiResult3::from((
            self.create_payment(context.get_lp_token_id(), 0, context.get_liquidity_added()),
            self.create_payment(
                context.get_first_token_id(),
                0,
                context.get_first_amount_optimal(),
            ),
            self.create_payment(
                context.get_second_token_id(),
                0,
                context.get_second_amount_optimal(),
            ),
        ))
    }

    fn construct_and_get_remove_liquidity_output_results(
        &self,
        context: &RemoveLiquidityContext<Self::Api>,
    ) -> RemoveLiquidityResultType<Self::Api> {
        MultiResult2::from((
            self.create_payment(
                context.get_first_token_id(),
                0,
                context.get_first_token_amount_removed(),
            ),
            self.create_payment(
                context.get_second_token_id(),
                0,
                context.get_second_token_amount_removed(),
            ),
        ))
    }
}
