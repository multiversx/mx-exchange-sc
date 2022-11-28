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
}
