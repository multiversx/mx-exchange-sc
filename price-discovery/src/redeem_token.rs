elrond_wasm::imports!();

pub const LAUNCHED_TOKEN_REDEEM_NONCE: u64 = 1;
pub const ACCEPTED_TOKEN_REDEEM_NONCE: u64 = 2;

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
    ) -> SCResult<AsyncCall> {
        require!(
            self.redeem_token_id().is_empty(),
            "Redeem token already issued"
        );

        Ok(self
            .send()
            .esdt_system_sc_proxy()
            .issue_fungible(
                payment_amount,
                &token_name,
                &token_ticker,
                &BigUint::zero(),
                FungibleTokenProperties {
                    can_add_special_roles: true,
                    can_burn: false,
                    can_change_owner: false,
                    can_freeze: false,
                    can_mint: false,
                    can_pause: false,
                    can_upgrade: false,
                    can_wipe: false,
                    num_decimals: nr_decimals,
                },
            )
            .async_call()
            .with_callback(self.callbacks().issue_callback()))
    }

    #[callback]
    fn issue_callback(&self, #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.redeem_token_id().set(&token_id);
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

    #[only_owner]
    #[endpoint(setLocalRoles)]
    fn set_local_roles(&self) -> AsyncCall {
        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &self.blockchain().get_sc_address(),
                &self.redeem_token_id().get(),
                (&[
                    EsdtLocalRole::NftCreate,
                    EsdtLocalRole::NftBurn,
                    EsdtLocalRole::NftAddQuantity,
                ][..])
                    .into_iter()
                    .cloned(),
            )
            .async_call()
            .with_callback(self.callbacks().set_roles_callback())
    }

    #[callback]
    fn set_roles_callback(&self, #[call_result] result: ManagedAsyncCallResult<()>) {
        match result {
            ManagedAsyncCallResult::Ok(()) => {
                // create SFT for both types so NFTAddQuantity works

                let redeem_token_id = self.redeem_token_id().get();
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
            ManagedAsyncCallResult::Err(_) => {}
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

    #[view(getRedeemTokenId)]
    #[storage_mapper("redeemTokenId")]
    fn redeem_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
