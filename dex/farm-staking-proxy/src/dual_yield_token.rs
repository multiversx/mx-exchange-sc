elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use hex_literal::hex;

const META_SFT_TOKEN_TYPE_NAME: &[u8] = b"META";
const ESDT_SYSTEM_SC_ADDRESS_ARRAY: [u8; 32] =
    hex!("000000000000000000010000000000000000000000000000000000000002ffff");

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct DualYieldTokenAttributes {
    pub farm_token_nonce: u64,
}

// temporary until added to Rust framework
mod esdt_system_sc {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait EsdtSystemSc {
        #[payable("EGLD")]
        #[endpoint(registerAndSetAllRoles)]
        fn register_and_set_all_roles(
            &self,
            #[payment_amount] payment_amount: BigUint,
            token_name: ManagedBuffer,
            token_ticker: ManagedBuffer,
            token_type: ManagedBuffer,
            num_decimals: usize,
        );
    }
}

#[elrond_wasm::module]
pub trait DualYieldTokenModule {
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueDualYieldToken)]
    fn issue_dual_yield_token(
        &self,
        #[payment_amount] payment_amount: BigUint,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) -> SCResult<AsyncCall> {
        require!(
            self.dual_yield_token_id().is_empty(),
            "Token already issued"
        );

        Ok(self
            .esdt_system_sc_proxy(ManagedAddress::new_from_bytes(
                &ESDT_SYSTEM_SC_ADDRESS_ARRAY,
            ))
            .register_and_set_all_roles(
                payment_amount,
                token_display_name,
                token_ticker,
                META_SFT_TOKEN_TYPE_NAME.into(),
                num_decimals,
            )
            .async_call()
            .with_callback(
                self.callbacks()
                    .issue_callback(&self.blockchain().get_caller()),
            ))
    }

    #[callback]
    fn issue_callback(
        &self,
        caller: &ManagedAddress,
        #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
    ) -> OptionalResult<ManagedBuffer> {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.dual_yield_token_id().set(&token_id);

                OptionalResult::None
            }
            ManagedAsyncCallResult::Err(err) => {
                let (returned_tokens, token_id) = self.call_value().payment_token_pair();
                if token_id.is_egld() && returned_tokens > 0 {
                    let _ = self.send().direct_egld(caller, &returned_tokens, &[]);
                }

                OptionalResult::Some(err.err_msg)
            }
        }
    }

    fn require_all_payments_dual_yield_tokens(
        &self,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> SCResult<()> {
        let dual_yield_token_id = self.dual_yield_token_id().get();
        for p in payments {
            require!(
                p.token_identifier == dual_yield_token_id,
                "Invalid payment token"
            );
        }

        Ok(())
    }

    fn create_dual_yield_tokens(&self, amount: &BigUint, farm_token_nonce: u64) -> u64 {
        let dual_yield_token_id = self.dual_yield_token_id().get();
        let empty_buffer = ManagedBuffer::new();

        self.send().esdt_nft_create(
            &dual_yield_token_id,
            amount,
            &empty_buffer,
            &BigUint::zero(),
            &empty_buffer,
            &DualYieldTokenAttributes { farm_token_nonce },
            &ManagedVec::new(),
        )
    }

    fn create_and_send_dual_yield_tokens(
        &self,
        to: &ManagedAddress,
        amount: &BigUint,
        farm_token_nonce: u64,
    ) {
        let new_token_nonce = self.create_dual_yield_tokens(amount, farm_token_nonce);
        let dual_yield_token_id = self.dual_yield_token_id().get();

        self.send()
            .direct(to, &dual_yield_token_id, new_token_nonce, amount, &[]);
    }

    fn burn_dual_yield_tokens(&self, sft_nonce: u64, amount: &BigUint) {
        let dual_yield_token_id = self.dual_yield_token_id().get();
        self.send()
            .esdt_local_burn(&dual_yield_token_id, sft_nonce, amount);
    }

    fn get_farm_token_nonce_from_attributes(&self, dual_yield_token_nonce: u64) -> SCResult<u64> {
        let dual_yield_token_id = self.dual_yield_token_id().get();
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            &dual_yield_token_id,
            dual_yield_token_nonce,
        );
        let attributes = token_info.decode_attributes::<DualYieldTokenAttributes>()?;

        Ok(attributes.farm_token_nonce)
    }

    #[proxy]
    fn esdt_system_sc_proxy(&self, sc_address: ManagedAddress) -> esdt_system_sc::Proxy<Self::Api>;

    #[view(getDualYieldTokenId)]
    #[storage_mapper("dualYieldTokenId")]
    fn dual_yield_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
