elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use hex_literal::hex;

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
    #[endpoint(issueDualYieldToken)]
    fn issue_dual_yield_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        require!(
            self.dual_yield_token_id().is_empty(),
            "Token already issued"
        );

        let payment_amount = self.call_value().egld_value();
        self.esdt_system_sc_proxy(ManagedAddress::new_from_bytes(
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
        )
        .call_and_exit()
    }

    #[callback]
    fn issue_callback(
        &self,
        caller: &ManagedAddress,
        #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
    ) -> OptionalValue<ManagedBuffer> {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.dual_yield_token_id().set(&token_id);

                OptionalValue::None
            }
            ManagedAsyncCallResult::Err(err) => {
                let (returned_tokens, token_id) = self.call_value().payment_token_pair();
                if token_id.is_egld() && returned_tokens > 0 {
                    let _ = self.send().direct_egld(caller, &returned_tokens, &[]);
                }

                OptionalValue::Some(err.err_msg)
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

    #[proxy]
    fn esdt_system_sc_proxy(&self, sc_address: ManagedAddress) -> esdt_system_sc::Proxy<Self::Api>;

    #[view(getDualYieldTokenId)]
    #[storage_mapper("dualYieldTokenId")]
    fn dual_yield_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
