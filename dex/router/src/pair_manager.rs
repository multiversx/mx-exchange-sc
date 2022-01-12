elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Nonce = u64;

use super::factory;
use super::state;

type SwapOperationType<ManagedTypeApi> = MultiArg4<
    ManagedAddress<ManagedTypeApi>,
    ManagedBuffer<ManagedTypeApi>,
    TokenIdentifier<ManagedTypeApi>,
    BigUint<ManagedTypeApi>,
>;

const ACCEPT_PAY_FUNC_NAME: &[u8] = b"acceptPay";
const SWAP_TOKENS_FIXED_INPUT_FUNC_NAME: &[u8] = b"swapTokensFixedInput";
const SWAP_TOKENS_FIXED_OUTPUT_FUNC_NAME: &[u8] = b"swapTokensFixedOutput";

use pair::config::ProxyTrait as _;
use pair::fee::ProxyTrait as _;

#[elrond_wasm::module]
pub trait PairManagerModule:
    state::StateModule + factory::FactoryModule + token_send::TokenSendModule
{
    #[only_owner]
    #[endpoint(setFeeOn)]
    fn set_fee_on(
        &self,
        pair_address: ManagedAddress,
        fee_to_address: ManagedAddress,
        fee_token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        self.check_is_pair_sc(&pair_address)?;

        self.pair_contract_proxy(pair_address)
            .set_fee_on(true, fee_to_address, fee_token)
            .execute_on_dest_context();

        Ok(())
    }

    #[only_owner]
    #[endpoint(setFeeOff)]
    fn set_fee_off(
        &self,
        pair_address: ManagedAddress,
        fee_to_address: ManagedAddress,
        fee_token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        self.check_is_pair_sc(&pair_address)?;

        self.pair_contract_proxy(pair_address)
            .set_fee_on(false, fee_to_address, fee_token)
            .execute_on_dest_context();

        Ok(())
    }

    #[payable("*")]
    #[endpoint(acceptPay)]
    fn accept_pay(&self) {}

    #[payable("*")]
    #[endpoint(multiPairSwap)]
    fn multi_pair_swap(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: BigUint,
        #[payment_nonce] nonce: Nonce,
        swap_operations: MultiArgVec<SwapOperationType<Self::Api>>,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        require!(nonce == 0, "Invalid nonce. Should be zero");
        require!(amount > 0u32, "Invalid amount. Should not be zero");
        require!(
            !swap_operations.is_empty(),
            "Invalid swap operations chain. Should not be empty"
        );

        let caller = self.blockchain().get_caller();
        let mut payments = ManagedVec::new();
        let mut last_payment = self.create_payment(&token_id, nonce, &amount);

        for entry in swap_operations.into_vec() {
            let (pair_address, function, token_wanted, amount_wanted) = entry.into_tuple();
            self.check_is_pair_sc(&pair_address)?;

            if function == ManagedBuffer::from(SWAP_TOKENS_FIXED_INPUT_FUNC_NAME) {
                last_payment = self.actual_swap_fixed_input(
                    pair_address,
                    last_payment.token_identifier.clone(),
                    last_payment.amount,
                    token_wanted,
                    amount_wanted,
                );
            } else if function == ManagedBuffer::from(SWAP_TOKENS_FIXED_OUTPUT_FUNC_NAME) {
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
                return sc_error!("Invalid function to call");
            }
        }

        payments.push(last_payment);
        self.send_multiple_tokens(&caller, &payments, opt_accept_funds_func)?;

        Ok(())
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
                amount_in,
                token_out,
                amount_out_min,
                OptionalArg::Some(ManagedBuffer::from(ACCEPT_PAY_FUNC_NAME)),
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
                amount_in_max,
                token_out,
                amount_out,
                OptionalArg::Some(ManagedBuffer::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after))
            .into_tuple()
    }

    #[only_owner]
    #[endpoint]
    fn pause(&self, address: ManagedAddress) -> SCResult<()> {
        if address == self.blockchain().get_sc_address() {
            self.state().set(&false);
        } else {
            self.check_is_pair_sc(&address)?;
            self.pair_contract_proxy(address)
                .pause()
                .execute_on_dest_context();
        }
        Ok(())
    }

    #[only_owner]
    #[endpoint]
    fn resume(&self, address: ManagedAddress) -> SCResult<()> {
        if address == self.blockchain().get_sc_address() {
            self.state().set(&true);
        } else {
            self.check_is_pair_sc(&address)?;
            self.pair_contract_proxy(address)
                .resume()
                .execute_on_dest_context();
        }
        Ok(())
    }

    fn pause_pair(&self, address: ManagedAddress) {
        self.pair_contract_proxy(address)
            .pause()
            .execute_on_dest_context();
    }

    fn resume_pair(&self, address: ManagedAddress) {
        self.pair_contract_proxy(address)
            .resume()
            .execute_on_dest_context();
    }

    fn get_lp_token_for_pair(&self, address: &ManagedAddress) -> TokenIdentifier {
        self.pair_contract_proxy(address.clone())
            .get_lp_token_identifier()
            .execute_on_dest_context()
    }

    fn set_lp_token_for_pair(&self, address: &ManagedAddress, token_id: &TokenIdentifier) {
        self.pair_contract_proxy(address.clone())
            .set_lp_token_identifier(token_id.clone())
            .execute_on_dest_context();
    }

    fn check_is_pair_sc(&self, pair_address: &ManagedAddress) -> SCResult<()> {
        require!(
            self.pair_map()
                .values()
                .any(|address| &address == pair_address),
            "Not a pair SC"
        );
        Ok(())
    }

    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;
}
