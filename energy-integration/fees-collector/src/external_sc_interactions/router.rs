use common_types::{PaymentsVec, Week};
use router_proxy::FunctionName;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod router_proxy {
    use common_types::PaymentsVec;

    multiversx_sc::imports!();

    pub type FunctionName<M> = ManagedBuffer<M>;
    pub type SwapOperationType<M> =
        MultiValue4<ManagedAddress<M>, FunctionName<M>, TokenIdentifier<M>, BigUint<M>>;

    #[multiversx_sc::proxy]
    pub trait RouterProxy {
        #[payable("*")]
        #[endpoint(multiPairSwap)]
        fn multi_pair_swap(
            &self,
            swap_operations: MultiValueEncoded<SwapOperationType<Self::Api>>,
        ) -> PaymentsVec<Self::Api>;
    }
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, Clone)]
pub struct SwapOperation<M: ManagedTypeApi> {
    pub pair_address: ManagedAddress<M>,
    pub function_name: FunctionName<M>,
    pub input_token_id: TokenIdentifier<M>,
    pub min_amount_out: BigUint<M>,
}

pub type SwapOperationArgs<M> = MultiValueEncoded<M, ManagedVec<M, SwapOperation<M>>>;
pub type SingleSwapOperationArg<M> = ManagedVec<M, SwapOperation<M>>;

#[multiversx_sc::module]
pub trait RouterInteractionsModule:
    crate::fees_accumulation::FeesAccumulationModule
    + crate::config::ConfigModule
    + crate::events::FeesCollectorEventsModule
    + week_timekeeping::WeekTimekeepingModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
    + multiversx_sc_modules::only_admin::OnlyAdminModule
{
    #[only_owner]
    #[endpoint(setRouterAddress)]
    fn set_router_address(&self, router_address: ManagedAddress) {
        self.require_sc_address(&router_address);

        self.router_address().set(router_address);
    }

    /// Swaps tokens to the base token (i.e. MEX).
    ///
    /// The first token must be a known token to the fees collector, and the very last token must be MEX.
    ///
    /// The fees collector uses the given pair paths through the router contract.
    #[only_admin]
    #[endpoint(swapTokenToBaseToken)]
    fn swap_token_to_base_token(&self, swap_operations: SwapOperationArgs<Self::Api>) {
        let current_week = self.get_current_week();
        let router_address = self.router_address().get();
        let base_token_id = self.get_base_token_id();
        let mut total_base_tokens = BigUint::zero();
        for swap_op in swap_operations {
            let payment = self.check_args_and_get_first_token_payment(
                current_week,
                &base_token_id,
                swap_op.clone(),
            );
            if payment.amount == 0 {
                continue;
            }

            let mut received_tokens =
                self.call_swap_through_router(router_address.clone(), payment, swap_op);
            require!(
                received_tokens.token_identifier == base_token_id,
                "Invalid tokens received from router"
            );

            self.burn_base_token(&mut received_tokens);

            total_base_tokens += received_tokens.amount;
        }

        self.accumulated_fees(current_week, &base_token_id)
            .update(|acc_fees| *acc_fees += total_base_tokens);
    }

    fn check_args_and_get_first_token_payment(
        &self,
        current_week: Week,
        base_token_id: &TokenIdentifier,
        swap_operation: SingleSwapOperationArg<Self::Api>,
    ) -> EsdtTokenPayment {
        let mut iter = swap_operation.into_iter();
        let opt_first_item = iter.next();
        require!(opt_first_item.is_some(), "No arguments provided");

        let first_item = unsafe { opt_first_item.unwrap_unchecked() };
        let last_item = match iter.last() {
            Some(item) => item,
            None => first_item.clone(),
        };

        require!(
            self.known_tokens().contains(&first_item.input_token_id),
            "Invalid first token"
        );

        require!(
            &last_item.input_token_id == base_token_id,
            "Invalid last token"
        );

        let token_amount = self
            .accumulated_fees(current_week, &first_item.input_token_id)
            .take();

        EsdtTokenPayment::new(first_item.input_token_id, 0, token_amount)
    }

    fn call_swap_through_router(
        &self,
        router_address: ManagedAddress,
        payment: EsdtTokenPayment,
        swap_operation: SingleSwapOperationArg<Self::Api>,
    ) -> EsdtTokenPayment {
        let mut args = MultiValueEncoded::new();
        for swap_op in &swap_operation {
            args.push(
                (
                    swap_op.pair_address,
                    swap_op.function_name,
                    swap_op.input_token_id,
                    swap_op.min_amount_out,
                )
                    .into(),
            )
        }

        let output_payments: PaymentsVec<Self::Api> = self
            .router_proxy(router_address)
            .multi_pair_swap(args)
            .esdt(payment)
            .execute_on_dest_context();
        require!(
            !output_payments.is_empty(),
            "No payments received from router"
        );

        unsafe { output_payments.iter().last().unwrap_unchecked() }
    }

    #[storage_mapper("routerAddress")]
    fn router_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[proxy]
    fn router_proxy(&self, sc_address: ManagedAddress) -> router_proxy::Proxy<Self::Api>;
}
