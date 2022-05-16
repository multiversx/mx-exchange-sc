elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use itertools::Itertools;

use crate::AddLiquidityResultType;
use crate::RemoveLiquidityResultType;
use crate::SwapTokensFixedInputResultType;
use crate::SwapTokensFixedOutputResultType;

use crate::contexts::add_liquidity::*;
use crate::contexts::base::*;
use crate::contexts::remove_liquidity::*;
use crate::contexts::swap::SwapArgs;
use crate::contexts::swap::SwapContext;
use crate::contexts::swap::SwapPayments;
use crate::contexts::swap::SwapTxInput;

#[elrond_wasm::module]
pub trait CtxHelper:
    crate::config::ConfigModule
    + token_send::TokenSendModule
    + crate::amm::AmmModule
    + crate::locking_wrapper::LockingWrapperModule
{
    fn new_add_liquidity_context(
        &self,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> AddLiquidityContext<Self::Api> {
        let caller = self.blockchain().get_caller();

        let payment_tuple: Option<(EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>)> =
            self.call_value()
                .all_esdt_transfers()
                .into_iter()
                .collect_tuple();
        let (first_payment, second_payment) = match payment_tuple {
            Some(tuple) => (Some(tuple.0), Some(tuple.1)),
            None => (None, None),
        };

        let args = AddLiquidityArgs::new(first_token_amount_min, second_token_amount_min);
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
    ) -> RemoveLiquidityContext<Self::Api> {
        let caller = self.blockchain().get_caller();

        let payment =
            EsdtTokenPayment::new(payment_token.clone(), payment_nonce, payment_amount.clone());
        let args = RemoveLiquidityArgs::new(first_token_amount_min, second_token_amount_min);
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
    ) -> SwapContext<Self::Api> {
        let caller = self.blockchain().get_caller();

        let payment =
            EsdtTokenPayment::new(payment_token.clone(), payment_nonce, payment_amount.clone());
        let args = SwapArgs::new(out_token_id, out_amount);
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

        payments.push(EsdtTokenPayment::new(
            context.get_lp_token_id().clone(),
            0,
            context.get_liquidity_added().clone(),
        ));
        payments.push(EsdtTokenPayment::new(
            context.get_first_token_id().clone(),
            0,
            &context.get_first_payment().amount - context.get_first_amount_optimal(),
        ));
        payments.push(EsdtTokenPayment::new(
            context.get_second_token_id().clone(),
            0,
            &context.get_second_payment().amount - context.get_second_amount_optimal(),
        ));

        context.set_output_payments(payments);
    }

    fn construct_remove_liquidity_output_payments(
        &self,
        context: &mut RemoveLiquidityContext<Self::Api>,
    ) {
        let mut payments: ManagedVec<EsdtTokenPayment<Self::Api>> = ManagedVec::new();

        payments.push(EsdtTokenPayment::new(
            context.get_first_token_id().clone(),
            0,
            context.get_first_token_amount_removed().clone(),
        ));
        payments.push(EsdtTokenPayment::new(
            context.get_second_token_id().clone(),
            0,
            context.get_second_token_amount_removed().clone(),
        ));

        context.set_output_payments(payments);
    }

    fn construct_swap_output_payments(&self, context: &mut SwapContext<Self::Api>) {
        let mut payments: ManagedVec<EsdtTokenPayment<Self::Api>> = ManagedVec::new();

        if self.should_generate_locked_asset() {
            let locked_asset = self.call_lock_tokens(context);
            context.set_locked_asset_output(locked_asset.clone());

            payments.push(locked_asset);
        } else {
            payments.push(EsdtTokenPayment::new(
                context.get_token_out().clone(),
                0,
                context.get_final_output_amount().clone(),
            ));
        }

        if context.get_final_input_amount() != context.get_amount_in() {
            payments.push(EsdtTokenPayment::new(
                context.get_token_in().clone(),
                0,
                context.get_amount_in() - context.get_final_input_amount(),
            ));
        }

        context.set_output_payments(payments);
    }

    fn execute_output_payments(&self, context: &dyn Context<Self::Api>) {
        self.send_multiple_tokens_if_not_zero(context.get_caller(), context.get_output_payments());
    }

    fn commit_changes(&self, context: &dyn Context<Self::Api>) {
        self.pair_reserve(context.get_first_token_id())
            .set(context.get_first_token_reserve());
        self.pair_reserve(context.get_second_token_id())
            .set(context.get_second_token_reserve());

        if context.get_lp_token_supply() != &0u64 {
            self.lp_token_supply().set(context.get_lp_token_supply());
        }
    }

    fn construct_and_get_add_liquidity_output_results(
        &self,
        context: &AddLiquidityContext<Self::Api>,
    ) -> AddLiquidityResultType<Self::Api> {
        MultiValue3::from((
            EsdtTokenPayment::new(
                context.get_lp_token_id().clone(),
                0,
                context.get_liquidity_added().clone(),
            ),
            EsdtTokenPayment::new(
                context.get_first_token_id().clone(),
                0,
                context.get_first_amount_optimal().clone(),
            ),
            EsdtTokenPayment::new(
                context.get_second_token_id().clone(),
                0,
                context.get_second_amount_optimal().clone(),
            ),
        ))
    }

    fn construct_and_get_remove_liquidity_output_results(
        &self,
        context: &RemoveLiquidityContext<Self::Api>,
    ) -> RemoveLiquidityResultType<Self::Api> {
        MultiValue2::from((
            EsdtTokenPayment::new(
                context.get_first_token_id().clone(),
                0,
                context.get_first_token_amount_removed().clone(),
            ),
            EsdtTokenPayment::new(
                context.get_second_token_id().clone(),
                0,
                context.get_second_token_amount_removed().clone(),
            ),
        ))
    }

    fn construct_and_get_swap_input_results(
        &self,
        context: &SwapContext<Self::Api>,
    ) -> SwapTokensFixedInputResultType<Self::Api> {
        match context.get_locked_asset_output() {
            Some(payment) => payment.clone(),
            None => EsdtTokenPayment::new(
                context.get_token_out().clone(),
                0,
                context.get_final_output_amount().clone(),
            ),
        }
    }

    fn construct_and_get_swap_output_results(
        &self,
        context: &SwapContext<Self::Api>,
    ) -> SwapTokensFixedOutputResultType<Self::Api> {
        let residuum = context.get_amount_in_max() - context.get_final_input_amount();

        let first_result = match context.get_locked_asset_output() {
            Some(payment) => payment.clone(),
            None => EsdtTokenPayment::new(
                context.get_token_out().clone(),
                0,
                context.get_final_output_amount().clone(),
            ),
        };

        (
            first_result,
            EsdtTokenPayment::new(context.get_token_in().clone(), 0, residuum),
        )
            .into()
    }
}
