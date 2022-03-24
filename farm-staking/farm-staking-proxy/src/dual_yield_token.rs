elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub struct DualYieldTokenAttributes<M: ManagedTypeApi> {
    pub lp_farm_token_nonce: u64,
    pub lp_farm_token_amount: BigUint<M>,
    pub staking_farm_token_nonce: u64,
    pub staking_farm_token_amount: BigUint<M>,
}

impl<M: ManagedTypeApi> DualYieldTokenAttributes<M> {
    /// dual yield tokens are always created with an amount equal to staking_farm_token_amount,
    /// so we just return this field instead of duplicating
    #[inline]
    pub fn get_total_dual_yield_tokens_for_position(&self) -> &BigUint<M> {
        &self.staking_farm_token_amount
    }
}

#[elrond_wasm::module]
pub trait DualYieldTokenModule: token_merge::TokenMergeModule {
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerDualYieldToken)]
    fn register_dual_yield_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        require!(
            self.dual_yield_token_id().is_empty(),
            "Token already issued"
        );

        let register_cost = self.call_value().egld_value();

        self.register_token(
            register_cost,
            token_display_name,
            token_ticker,
            num_decimals,
        )
    }

    fn register_token(
        &self,
        register_cost: BigUint,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        self.send()
            .esdt_system_sc_proxy()
            .register_meta_esdt(
                register_cost,
                &token_display_name,
                &token_ticker,
                MetaTokenProperties {
                    num_decimals,
                    can_freeze: true,
                    can_wipe: true,
                    can_pause: true,
                    can_change_owner: true,
                    can_upgrade: true,
                    can_add_special_roles: true,
                },
            )
            .async_call()
            .with_callback(
                self.callbacks()
                    .register_callback(&self.blockchain().get_caller()),
            )
            .call_and_exit()
    }

    #[callback]
    fn register_callback(
        &self,
        caller: &ManagedAddress,
        #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.last_error_message().clear();

                self.dual_yield_token_id().set_if_empty(&token_id);
            }
            ManagedAsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);

                let (returned_tokens, token_id) = self.call_value().payment_token_pair();
                if token_id.is_egld() && returned_tokens > 0 {
                    let _ = self.send().direct_egld(caller, &returned_tokens, &[]);
                }
            }
        }
    }

    #[only_owner]
    #[endpoint(setLocalRolesDualYieldToken)]
    fn set_local_roles_dual_yield_token(&self) {
        require!(!self.dual_yield_token_id().is_empty(), "No farm token");

        let token = self.dual_yield_token_id().get();
        self.set_local_roles(token)
    }

    fn set_local_roles(&self, token: TokenIdentifier) {
        let roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ];

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &self.blockchain().get_sc_address(),
                &token,
                roles.iter().cloned(),
            )
            .async_call()
            .with_callback(self.callbacks().change_roles_callback())
            .call_and_exit()
    }

    #[callback]
    fn change_roles_callback(&self, #[call_result] result: ManagedAsyncCallResult<()>) {
        match result {
            ManagedAsyncCallResult::Ok(()) => {
                self.last_error_message().clear();
            }
            ManagedAsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);
            }
        }
    }

    fn require_dual_yield_token(&self, token_id: &TokenIdentifier) {
        let dual_yield_token_id = self.dual_yield_token_id().get();
        require!(token_id == &dual_yield_token_id, "Invalid payment token");
    }

    fn require_all_payments_dual_yield_tokens(
        &self,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) {
        if payments.is_empty() {
            return;
        }

        let dual_yield_token_id = self.dual_yield_token_id().get();
        for p in payments {
            require!(
                p.token_identifier == dual_yield_token_id,
                "Invalid payment token"
            );
        }
    }

    fn create_and_send_dual_yield_tokens(
        &self,
        to: &ManagedAddress,
        lp_farm_token_nonce: u64,
        lp_farm_token_amount: BigUint,
        staking_farm_token_nonce: u64,
        staking_farm_token_amount: BigUint,
    ) -> EsdtTokenPayment<Self::Api> {
        let payment = self.create_dual_yield_tokens(
            lp_farm_token_nonce,
            lp_farm_token_amount,
            staking_farm_token_nonce,
            staking_farm_token_amount,
        );
        self.send().direct(
            to,
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
            &[],
        );

        payment
    }

    fn create_dual_yield_tokens(
        &self,
        lp_farm_token_nonce: u64,
        lp_farm_token_amount: BigUint,
        staking_farm_token_nonce: u64,
        staking_farm_token_amount: BigUint,
    ) -> EsdtTokenPayment<Self::Api> {
        let dual_yield_token_id = self.dual_yield_token_id().get();
        let empty_buffer = ManagedBuffer::new();
        let attributes = DualYieldTokenAttributes {
            lp_farm_token_nonce,
            lp_farm_token_amount,
            staking_farm_token_nonce,
            staking_farm_token_amount,
        };
        let amount = attributes.get_total_dual_yield_tokens_for_position();
        let new_token_nonce = self.send().esdt_nft_create(
            &dual_yield_token_id,
            amount,
            &empty_buffer,
            &BigUint::zero(),
            &empty_buffer,
            &attributes,
            &ManagedVec::new(),
        );

        EsdtTokenPayment::new(dual_yield_token_id, new_token_nonce, amount.clone())
    }

    fn burn_dual_yield_tokens(&self, sft_nonce: u64, amount: &BigUint) {
        let dual_yield_token_id = self.dual_yield_token_id().get();
        self.send()
            .esdt_local_burn(&dual_yield_token_id, sft_nonce, amount);
    }

    fn get_dual_yield_token_attributes(
        &self,
        dual_yield_token_nonce: u64,
    ) -> DualYieldTokenAttributes<Self::Api> {
        let own_sc_address = self.blockchain().get_sc_address();
        let dual_yield_token_id = self.dual_yield_token_id().get();
        let token_info = self.blockchain().get_esdt_token_data(
            &own_sc_address,
            &dual_yield_token_id,
            dual_yield_token_nonce,
        );

        token_info.decode_attributes()
    }

    fn get_lp_farm_token_amount_equivalent(
        &self,
        attributes: &DualYieldTokenAttributes<Self::Api>,
        amount: &BigUint,
    ) -> BigUint {
        self.rule_of_three_non_zero_result(
            amount,
            attributes.get_total_dual_yield_tokens_for_position(),
            &attributes.lp_farm_token_amount,
        )
    }

    #[inline]
    fn get_staking_farm_token_amount_equivalent(&self, amount: &BigUint) -> BigUint {
        // since staking_farm_token_amount is equal to the total dual yield tokens,
        // we simply return the amount
        amount.clone()
    }

    #[view(getDualYieldTokenId)]
    #[storage_mapper("dualYieldTokenId")]
    fn dual_yield_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<ManagedBuffer>;
}
