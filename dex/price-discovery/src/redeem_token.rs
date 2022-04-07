elrond_wasm::imports!();

pub const LAUNCHED_TOKEN_REDEEM_NONCE: u64 = 1;
pub const ACCEPTED_TOKEN_REDEEM_NONCE: u64 = 2;

const REQUIRED_ROLES: EsdtLocalRoleFlags = EsdtLocalRoleFlags::from_bits_truncate(
    EsdtLocalRoleFlags::NFT_CREATE.bits()
        | EsdtLocalRoleFlags::NFT_ADD_QUANTITY.bits()
        | EsdtLocalRoleFlags::NFT_BURN.bits(),
);

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

        ESDTSystemSmartContractProxy::new_proxy_obj()
            .register_meta_esdt(
                payment_amount,
                &token_name,
                &token_ticker,
                MetaTokenProperties {
                    num_decimals: nr_decimals,
                    can_freeze: true,
                    can_wipe: true,
                    can_pause: true,
                    can_change_owner: false,
                    can_upgrade: false,
                    can_add_special_roles: true,
                },
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
    #[endpoint(setLocalRoles)]
    fn set_local_roles(&self, token_id: TokenIdentifier) {
        require!(!self.redeem_token_id().is_empty(), "Token not issed");

        ESDTSystemSmartContractProxy::new_proxy_obj()
            .set_special_roles(
                &self.blockchain().get_sc_address(),
                &token_id,
                REQUIRED_ROLES.iter_roles().cloned(),
            )
            .async_call()
            .with_callback(self.callbacks().set_roles_callback())
            .call_and_exit();
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
