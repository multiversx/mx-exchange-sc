elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::RemoveLiquidityResultType;
use crate::SwapTokensFixedInputResultType;
use crate::SwapTokensFixedOutputResultType;

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
    + pausable::PausableModule
{
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
