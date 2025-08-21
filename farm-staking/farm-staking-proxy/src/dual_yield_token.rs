use fixed_supply_token::FixedSupplyToken;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(TopEncode, TopDecode, Clone, PartialEq, Debug)]
pub struct DualYieldTokenAttributes<M: ManagedTypeApi> {
    pub lp_farm_token_nonce: u64,
    pub lp_farm_token_amount: BigUint<M>,
    pub staking_farm_token_nonce: u64,
    pub staking_farm_token_amount: BigUint<M>,
}

impl<M: ManagedTypeApi> FixedSupplyToken<M> for DualYieldTokenAttributes<M> {
    fn get_total_supply(&self) -> BigUint<M> {
        self.staking_farm_token_amount.clone()
    }

    fn into_part(self, payment_amount: &BigUint<M>) -> Self {
        if payment_amount == &self.get_total_supply() {
            return self;
        }

        let new_lp_farm_token_amount =
            self.rule_of_three_non_zero_result(payment_amount, &self.lp_farm_token_amount);
        let new_staking_farm_token_amount = payment_amount.clone();

        DualYieldTokenAttributes {
            lp_farm_token_nonce: self.lp_farm_token_nonce,
            lp_farm_token_amount: new_lp_farm_token_amount,
            staking_farm_token_nonce: self.staking_farm_token_nonce,
            staking_farm_token_amount: new_staking_farm_token_amount,
        }
    }
}

#[multiversx_sc::module]
pub trait DualYieldTokenModule:
    multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
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
        let register_cost = self.call_value().egld().clone_value();
        self.dual_yield_token().issue_and_set_all_roles(
            EsdtTokenType::MetaFungible,
            register_cost,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    fn create_dual_yield_tokens(
        &self,
        mapper: &NonFungibleTokenMapper,
        attributes: &DualYieldTokenAttributes<Self::Api>,
    ) -> EsdtTokenPayment {
        let new_dual_yield_amount = attributes.get_total_supply();
        mapper.nft_create(new_dual_yield_amount, attributes)
    }

    #[view(getDualYieldTokenId)]
    #[storage_mapper("dualYieldTokenId")]
    fn dual_yield_token(&self) -> NonFungibleTokenMapper;
}
