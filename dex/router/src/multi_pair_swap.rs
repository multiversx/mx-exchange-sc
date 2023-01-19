multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use super::factory;

use pair::ProxyTrait as _;

type SwapOperationType<M> =
    MultiValue4<ManagedAddress<M>, ManagedBuffer<M>, TokenIdentifier<M>, BigUint<M>>;

pub const SWAP_TOKENS_FIXED_INPUT_FUNC_NAME: &[u8] = b"swapTokensFixedInput";
pub const SWAP_TOKENS_FIXED_OUTPUT_FUNC_NAME: &[u8] = b"swapTokensFixedOutput";

#[multiversx_sc::module]
pub trait MultiPairSwap: factory::FactoryModule + token_send::TokenSendModule {
    #[payable("*")]
    #[endpoint(multiPairSwap)]
    fn multi_pair_swap(&self, swap_operations: MultiValueEncoded<SwapOperationType<Self::Api>>) {
        let (token_id, nonce, amount) = self.call_value().single_esdt().into_tuple();
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

        for entry in swap_operations.into_iter() {
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
        self.send().direct_multi(&caller, &payments);
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
            .swap_tokens_fixed_input(token_out, amount_out_min)
            .with_esdt_transfer((token_in, 0, amount_in))
            .execute_on_dest_context()
    }

    fn actual_swap_fixed_output(
        &self,
        pair_address: ManagedAddress,
        token_in: TokenIdentifier,
        amount_in_max: BigUint,
        token_out: TokenIdentifier,
        amount_out: BigUint,
    ) -> (EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>) {
        let call_result: MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> =
            self.pair_contract_proxy(pair_address)
                .swap_tokens_fixed_output(token_out, amount_out)
                .with_esdt_transfer((token_in, 0, amount_in_max))
                .execute_on_dest_context();

        call_result.into_tuple()
    }

    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;
}
