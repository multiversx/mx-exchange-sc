elrond_wasm::imports!();
use hex_literal::hex;

pub const LAUNCHED_TOKEN_REDEEM_NONCE: u64 = 1;
pub const ACCEPTED_TOKEN_REDEEM_NONCE: u64 = 2;

const META_SFT_TOKEN_TYPE_NAME: &[u8] = b"META";
const ESDT_SYSTEM_SC_ADDRESS_ARRAY: [u8; 32] =
    hex!("000000000000000000010000000000000000000000000000000000000002ffff");

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
pub trait RedeemTokenModule {
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueRedeemToken)]
    fn issue_redeem_token(
        &self,
        #[payment_amount] payment_amount: BigUint,
        token_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        nr_decimals: usize,
    ) {
        require!(
            self.redeem_token_id().is_empty(),
            "Redeem token already issued"
        );

        self.esdt_system_sc_proxy(ManagedAddress::new_from_bytes(
            &ESDT_SYSTEM_SC_ADDRESS_ARRAY,
        ))
        .register_and_set_all_roles(
            payment_amount,
            token_name,
            token_ticker,
            META_SFT_TOKEN_TYPE_NAME.into(),
            nr_decimals,
        )
        .async_call()
        .with_callback(self.callbacks().issue_callback())
        .call_and_exit()
    }

    #[callback]
    fn issue_callback(&self, #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>) {
        match result {
            ManagedAsyncCallResult::Ok(redeem_token_id) => {
                self.redeem_token_id().set(&redeem_token_id);

                // create SFT for both types so NFTAddQuantity works
                let zero = BigUint::zero();
                let one = BigUint::from(1u32);
                let empty_buffer = ManagedBuffer::new();
                let empty_vec = ManagedVec::new();

                let _ = self.send().esdt_nft_create(
                    &redeem_token_id,
                    &one,
                    &empty_buffer,
                    &zero,
                    &empty_buffer,
                    &(),
                    &empty_vec,
                );
                let _ = self.send().esdt_nft_create(
                    &redeem_token_id,
                    &one,
                    &empty_buffer,
                    &zero,
                    &empty_buffer,
                    &(),
                    &empty_vec,
                );
            }
            ManagedAsyncCallResult::Err(_) => {
                let caller = self.blockchain().get_owner_address();
                let (returned_tokens, token_id) = self.call_value().payment_token_pair();
                if token_id.is_egld() && returned_tokens > 0 {
                    self.send()
                        .direct(&caller, &token_id, 0, &returned_tokens, &[]);
                }
            }
        }
    }

    fn mint_and_send_redeem_token(&self, to: &ManagedAddress, nonce: u64, amount: &BigUint) {
        let redeem_token_id = self.redeem_token_id().get();
        self.send().esdt_local_mint(&redeem_token_id, nonce, amount);
        self.send().direct(to, &redeem_token_id, nonce, amount, &[]);
    }

    fn burn_redeem_token(&self, nonce: u64, amount: &BigUint) {
        let redeem_token_id = self.redeem_token_id().get();
        self.send().esdt_local_burn(&redeem_token_id, nonce, amount);
    }

    #[proxy]
    fn esdt_system_sc_proxy(&self, sc_address: ManagedAddress) -> esdt_system_sc::Proxy<Self::Api>;

    #[view(getRedeemTokenId)]
    #[storage_mapper("redeemTokenId")]
    fn redeem_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
