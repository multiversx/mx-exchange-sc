elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::factory;

use pair::ProxyTrait as _;

type SwapOperationType<M> =
    MultiValue4<ManagedAddress<M>, ManagedBuffer<M>, TokenIdentifier<M>, BigUint<M>>;

pub const SWAP_TOKENS_FIXED_INPUT_FUNC_NAME: &[u8] = b"swapTokensFixedInput";
pub const SWAP_TOKENS_FIXED_OUTPUT_FUNC_NAME: &[u8] = b"swapTokensFixedOutput";

#[elrond_wasm::module]
pub trait Lib: factory::FactoryModule + token_send::TokenSendModule {
    #[payable("*")]
    #[endpoint(multiPairSwap)]
    fn multi_pair_swap(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: BigUint,
        #[payment_nonce] nonce: u64,
        swap_operations: MultiValueVec<SwapOperationType<Self::Api>>,
        #[var_args] opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) {
        require!(nonce == 0, "Invalid nonce. Should be zero");
        require!(amount > 0u64, "Invalid amount. Should not be zero");
        require!(
            !swap_operations.is_empty(),
            "Invalid swap operations chain. Should not be empty"
        );

        let swap_fixed_input_endpoint = ManagedBuffer::from(SWAP_TOKENS_FIXED_INPUT_FUNC_NAME);
        let swap_fixed_output_endpoint = ManagedBuffer::from(SWAP_TOKENS_FIXED_OUTPUT_FUNC_NAME);

        let caller = self.blockchain().get_caller();
        let mut payments = ManagedVec::new();
        let mut last_payment = EsdtTokenPayment::new(token_id, nonce, amount);

        for entry in swap_operations.into_vec() {
            let (pair_address, function, token_wanted, amount_wanted) = entry.into_tuple();
            self.check_is_pair_sc(&pair_address);

            if function == swap_fixed_input_endpoint {
                last_payment = self.actual_swap_fixed_input(
                    pair_address,
                    last_payment.token_identifier,
                    last_payment.amount,
                    token_wanted,
                    amount_wanted,
                );
            } else if function == swap_fixed_output_endpoint {
                let (payment, residuum) = self.actual_swap_fixed_output(
                    pair_address,
                    last_payment.token_identifier,
                    last_payment.amount,
                    token_wanted,
                    amount_wanted,
                );

                last_payment = payment;
                payments.push(residuum);
            } else {
                sc_panic!("Invalid function to call");
            }
        }

        payments.push(last_payment);
        self.send_multiple_tokens(&caller, &payments, &opt_accept_funds_func);
    }

    fn actual_swap_fixed_input(
        &self,
        pair_address: ManagedAddress,
        token_in: TokenIdentifier,
        amount_in: BigUint,
        token_out: TokenIdentifier,
        amount_out_min: BigUint,
    ) -> EsdtTokenPayment<Self::Api> {
        self.pair_contract_proxy(pair_address)
            .swap_tokens_fixed_input(
                token_in,
                0,
                amount_in,
                token_out,
                amount_out_min,
                OptionalValue::None,
            )
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after))
    }

    fn actual_swap_fixed_output(
        &self,
        pair_address: ManagedAddress,
        token_in: TokenIdentifier,
        amount_in_max: BigUint,
        token_out: TokenIdentifier,
        amount_out: BigUint,
    ) -> (EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>) {
        self.pair_contract_proxy(pair_address)
            .swap_tokens_fixed_output(
                token_in,
                0,
                amount_in_max,
                token_out,
                amount_out,
                OptionalValue::None,
            )
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after))
            .into_tuple()
    }

    fn check_is_pair_sc(&self, pair_address: &ManagedAddress) {
        require!(
            self.pair_map()
                .values()
                .any(|address| &address == pair_address),
            "Not a pair SC"
        );
    }

    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;
}
