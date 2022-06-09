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
pub trait DualYieldTokenModule:
    token_merge::TokenMergeModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerDualYieldToken)]
    fn register_dual_yield_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        let register_cost = self.call_value().egld_value();
        self.dual_yield_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            register_cost,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
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
        self.send().direct_esdt(
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
        let attributes = DualYieldTokenAttributes {
            lp_farm_token_nonce,
            lp_farm_token_amount,
            staking_farm_token_nonce,
            staking_farm_token_amount,
        };
        let amount = attributes.get_total_dual_yield_tokens_for_position();

        self.dual_yield_token()
            .nft_create(amount.clone(), &attributes)
    }

    #[inline]
    fn burn_dual_yield_tokens(&self, sft_nonce: u64, amount: &BigUint) {
        self.dual_yield_token().nft_burn(sft_nonce, amount)
    }

    #[inline]
    fn get_dual_yield_token_attributes(
        &self,
        dual_yield_token_nonce: u64,
    ) -> DualYieldTokenAttributes<Self::Api> {
        self.dual_yield_token()
            .get_token_attributes(dual_yield_token_nonce)
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
    fn dual_yield_token(&self) -> NonFungibleTokenMapper<Self::Api>;
}
