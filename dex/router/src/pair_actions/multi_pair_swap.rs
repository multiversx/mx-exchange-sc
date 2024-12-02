multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use pair::{pair_actions::swap::ProxyTrait as _, read_pair_storage};

type SwapOperationType<M> =
    MultiValue4<ManagedAddress<M>, ManagedBuffer<M>, TokenIdentifier<M>, BigUint<M>>;

pub static SWAP_TOKENS_FIXED_INPUT_FUNC_NAME: &[u8] = b"swapTokensFixedInput";
pub static SWAP_TOKENS_FIXED_OUTPUT_FUNC_NAME: &[u8] = b"swapTokensFixedOutput";

pub struct SwapFixedInputArgs<M: ManagedTypeApi> {
    pub pair_address: ManagedAddress<M>,
    pub token_in: TokenIdentifier<M>,
    pub amount_in: BigUint<M>,
    pub token_out: TokenIdentifier<M>,
    pub amount_out_min: BigUint<M>,
}

pub struct SwapFixedOutputArgs<M: ManagedTypeApi> {
    pub pair_address: ManagedAddress<M>,
    pub token_in: TokenIdentifier<M>,
    pub amount_in_max: BigUint<M>,
    pub token_out: TokenIdentifier<M>,
    pub amount_out: BigUint<M>,
}

#[multiversx_sc::module]
pub trait MultiPairSwap:
    crate::config::ConfigModule
    + read_pair_storage::ReadPairStorageModule
    + token_send::TokenSendModule
    + crate::events::EventsModule
    + crate::state::StateModule
{
    #[payable("*")]
    #[endpoint(multiPairSwap)]
    fn multi_pair_swap(
        &self,
        swap_operations: MultiValueEncoded<SwapOperationType<Self::Api>>,
    ) -> ManagedVec<EsdtTokenPayment> {
        self.require_active();

        let (token_id, amount) = self.call_value().single_fungible_esdt();
        require!(amount > 0u64, "Invalid amount. Should not be zero");
        require!(
            !swap_operations.is_empty(),
            "Invalid swap operations chain. Should not be empty"
        );

        let swap_fixed_input_endpoint = ManagedBuffer::from(SWAP_TOKENS_FIXED_INPUT_FUNC_NAME);
        let swap_fixed_output_endpoint = ManagedBuffer::from(SWAP_TOKENS_FIXED_OUTPUT_FUNC_NAME);

        let caller = self.blockchain().get_caller();
        let mut payments = ManagedVec::new();
        let mut last_payment = EsdtTokenPayment::new(token_id.clone(), 0, amount.clone());

        for entry in swap_operations.into_iter() {
            let (pair_address, function, token_wanted, amount_wanted) = entry.into_tuple();
            self.check_is_pair_sc(&pair_address);

            if function == swap_fixed_input_endpoint {
                last_payment = self.actual_swap_fixed_input(SwapFixedInputArgs {
                    pair_address,
                    token_in: last_payment.token_identifier,
                    amount_in: last_payment.amount,
                    token_out: token_wanted,
                    amount_out_min: amount_wanted,
                });
            } else if function == swap_fixed_output_endpoint {
                let (payment, residuum) = self.actual_swap_fixed_output(SwapFixedOutputArgs {
                    pair_address,
                    token_in: last_payment.token_identifier,
                    amount_in_max: last_payment.amount,
                    token_out: token_wanted,
                    amount_out: amount_wanted,
                });
                last_payment = payment;

                if residuum.amount > 0 {
                    payments.push(residuum);
                }
            } else {
                sc_panic!("Invalid function to call");
            }
        }

        payments.push(last_payment);
        self.send().direct_multi(&caller, &payments);

        self.emit_multi_pair_swap_event(caller, token_id, amount, payments.clone());

        payments
    }

    fn actual_swap_fixed_input(&self, args: SwapFixedInputArgs<Self::Api>) -> EsdtTokenPayment {
        self.pair_contract_proxy(args.pair_address)
            .swap_tokens_fixed_input(args.token_out, args.amount_out_min)
            .with_esdt_transfer((args.token_in, 0, args.amount_in))
            .execute_on_dest_context()
    }

    fn actual_swap_fixed_output(
        &self,
        args: SwapFixedOutputArgs<Self::Api>,
    ) -> (EsdtTokenPayment, EsdtTokenPayment) {
        let call_result: MultiValue2<EsdtTokenPayment, EsdtTokenPayment> = self
            .pair_contract_proxy(args.pair_address)
            .swap_tokens_fixed_output(args.token_out, args.amount_out)
            .with_esdt_transfer((args.token_in, 0, args.amount_in_max))
            .execute_on_dest_context();

        call_result.into_tuple()
    }

    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;
}
