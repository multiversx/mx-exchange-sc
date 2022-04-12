elrond_wasm::imports!();

pub const LAUNCHED_TOKEN_REDEEM_NONCE: u64 = 1;
pub const ACCEPTED_TOKEN_REDEEM_NONCE: u64 = 2;

#[elrond_wasm::module]
pub trait RedeemTokenModule: crate::common_storage::CommonStorageModule {
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

        ESDTSystemSmartContractProxy::new_proxy_obj()
            .issue_and_set_all_roles(
                payment_amount,
                token_name,
                token_ticker,
                EsdtTokenType::Meta,
                nr_decimals,
            )
            .async_call()
            .with_callback(self.callbacks().issue_callback())
            .call_and_exit();
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
    #[endpoint(createInitialRedeemTokens)]
    fn create_initial_redeem_tokens(&self) {
        require!(!self.redeem_token_id().is_empty(), "Token not issued");

        // create SFT for both types so NFTAddQuantity works
        let launched_token_id = self.launched_token_id().get();
        let accpeted_token_id = self.accepted_token_id().get();
        let redeem_token_id = self.redeem_token_id().get();
        let zero = BigUint::zero();
        let one = BigUint::from(1u32);
        let empty_buffer = ManagedBuffer::new();
        let empty_vec = ManagedVec::new();

        let _ = self.send().esdt_nft_create(
            &redeem_token_id,
            &one,
            launched_token_id.as_managed_buffer(),
            &zero,
            &empty_buffer,
            &(),
            &empty_vec,
        );
        let _ = self.send().esdt_nft_create(
            &redeem_token_id,
            &one,
            accpeted_token_id.as_managed_buffer(),
            &zero,
            &empty_buffer,
            &(),
            &empty_vec,
        );
    }

    fn mint_and_send_redeem_token(&self, to: &ManagedAddress, nonce: u64, amount: &BigUint) {
        let redeem_token_id = self.redeem_token_id().get();
        self.send().esdt_local_mint(&redeem_token_id, nonce, amount);

        self.redeem_token_total_circulating_supply(nonce)
            .update(|supply| *supply += amount);

        self.send().direct(to, &redeem_token_id, nonce, amount, &[]);
    }

    fn burn_redeem_token(&self, nonce: u64, amount: &BigUint) {
        self.burn_redeem_token_without_supply_decrease(nonce, amount);

        self.redeem_token_total_circulating_supply(nonce)
            .update(|supply| *supply -= amount);
    }

    fn burn_redeem_token_without_supply_decrease(&self, nonce: u64, amount: &BigUint) {
        let redeem_token_id = self.redeem_token_id().get();
        self.send().esdt_local_burn(&redeem_token_id, nonce, amount);
    }

    #[view(getRedeemTokenId)]
    #[storage_mapper("redeemTokenId")]
    fn redeem_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("totalCirculatingSupply")]
    fn redeem_token_total_circulating_supply(&self, token_nonce: u64)
        -> SingleValueMapper<BigUint>;
}
