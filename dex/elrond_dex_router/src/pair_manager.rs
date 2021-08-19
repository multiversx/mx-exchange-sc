elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::FftTokenAmountPair;

use super::factory;
use super::state;

type Nonce = u64;
type SwapOperationType<BigUint> = MultiArg4<Address, BoxedBytes, TokenIdentifier, BigUint>;

const ACCEPT_PAY_FUNC_NAME: &[u8] = b"acceptPay";
const SWAP_TOKENS_FIXED_INPUT_FUNC_NAME: &[u8] = b"swapTokensFixedInput";
const SWAP_TOKENS_FIXED_OUTPUT_FUNC_NAME: &[u8] = b"swapTokensFixedOutput";

use elrond_dex_pair::config::ProxyTrait as _;
use elrond_dex_pair::fee::ProxyTrait as _;

#[elrond_wasm::module]
pub trait PairManagerModule:
    state::StateModule + factory::FactoryModule + token_send::TokenSendModule
{
    #[only_owner]
    #[endpoint(setFeeOn)]
    fn set_fee_on(
        &self,
        pair_address: Address,
        fee_to_address: Address,
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
        pair_address: Address,
        fee_to_address: Address,
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
        #[payment_amount] amount: Self::BigUint,
        #[payment_nonce] nonce: Nonce,
        swap_operations: MultiArgVec<SwapOperationType<Self::BigUint>>,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        require!(nonce == 0, "Invalid nonce. Should be zero");
        require!(amount > 0, "Invalid amount. Should not be zero");
        require!(
            !swap_operations.is_empty(),
            "Invalid swap operations chain. Should not be empty"
        );

        let caller = self.blockchain().get_caller();
        let mut residuum_vec = Vec::new();
        let mut last_received_token_id = token_id;
        let mut last_received_amount = amount;

        for entry in swap_operations.into_vec() {
            let (pair_address, function, token_wanted, amount_wanted) = entry.into_tuple();
            self.check_is_pair_sc(&pair_address)?;

            if function == BoxedBytes::from(SWAP_TOKENS_FIXED_INPUT_FUNC_NAME) {
                let token_amount_out = self.actual_swap_fixed_input(
                    pair_address,
                    last_received_token_id,
                    last_received_amount,
                    token_wanted,
                    amount_wanted,
                );
                last_received_token_id = token_amount_out.token_id;
                last_received_amount = token_amount_out.amount;
            } else if function == BoxedBytes::from(SWAP_TOKENS_FIXED_OUTPUT_FUNC_NAME) {
                let (token_amount_out, residuum) = self.actual_swap_fixed_output(
                    pair_address,
                    last_received_token_id,
                    last_received_amount,
                    token_wanted,
                    amount_wanted,
                );
                last_received_token_id = token_amount_out.token_id;
                last_received_amount = token_amount_out.amount;
                residuum_vec.push(residuum);
            } else {
                return sc_error!("Invalid function to call");
            }
        }

        while !residuum_vec.is_empty() {
            let residuum = residuum_vec.pop().unwrap_or(FftTokenAmountPair {
                token_id: TokenIdentifier::from(BoxedBytes::empty()),
                amount: Self::BigUint::zero(),
            });
            self.send_fft_tokens(
                &residuum.token_id,
                &residuum.amount,
                &caller,
                &opt_accept_funds_func,
            )?;
        }

        self.send_fft_tokens(
            &last_received_token_id,
            &last_received_amount,
            &caller,
            &opt_accept_funds_func,
        )?;

        Ok(())
    }

    fn actual_swap_fixed_input(
        &self,
        pair_address: Address,
        token_in: TokenIdentifier,
        amount_in: Self::BigUint,
        token_out: TokenIdentifier,
        amount_out_min: Self::BigUint,
    ) -> FftTokenAmountPair<Self::BigUint> {
        self.pair_contract_proxy(pair_address)
            .swap_tokens_fixed_input(
                token_in,
                amount_in,
                token_out,
                amount_out_min,
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after))
    }

    fn actual_swap_fixed_output(
        &self,
        pair_address: Address,
        token_in: TokenIdentifier,
        amount_in_max: Self::BigUint,
        token_out: TokenIdentifier,
        amount_out: Self::BigUint,
    ) -> (
        FftTokenAmountPair<Self::BigUint>,
        FftTokenAmountPair<Self::BigUint>,
    ) {
        self.pair_contract_proxy(pair_address)
            .swap_tokens_fixed_output(
                token_in,
                amount_in_max,
                token_out,
                amount_out,
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after))
            .into_tuple()
    }

    fn pause_pair(&self, address: Address) {
        self.pair_contract_proxy(address)
            .pause()
            .execute_on_dest_context();
    }

    fn resume_pair(&self, address: Address) {
        self.pair_contract_proxy(address)
            .resume()
            .execute_on_dest_context();
    }

    fn get_lp_token_for_pair(&self, address: &Address) -> TokenIdentifier {
        self.pair_contract_proxy(address.clone())
            .get_lp_token_identifier()
            .execute_on_dest_context()
    }

    fn set_lp_token_for_pair(&self, address: &Address, token_id: &TokenIdentifier) {
        self.pair_contract_proxy(address.clone())
            .set_lp_token_identifier(token_id.clone())
            .execute_on_dest_context();
    }

    #[proxy]
    fn pair_contract_proxy(&self, to: Address) -> elrond_dex_pair::Proxy<Self::SendApi>;
}
