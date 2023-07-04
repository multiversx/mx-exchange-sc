multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, PartialEq, Eq, TypeAbi, Clone)]
pub struct LpTokenAttributes<M: ManagedTypeApi> {
    pub virtual_liquidity: BigUint<M>,
    pub tick_min: i32,
    pub tick_max: i32,
    pub first_token_accumulated_fee: BigUint<M>,
    pub second_token_accumulated_fee: BigUint<M>,
}

#[multiversx_sc::module]
pub trait LpTokenModule:
    multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerLpToken)]
    fn register_lp_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        let payment_amount = self.call_value().egld_value();
        self.lp_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            payment_amount,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    fn mint_lp_tokens<T: TopEncode>(
        &self,
        amount: BigUint,
        attributes: &T,
    ) -> EsdtTokenPayment<Self::Api> {
        // self.lp_token_supply().update(|x| *x += &amount);
        self.lp_token().nft_create(amount, attributes)
    }

    fn burn_lp_tokens(&self, nonce: u64, amount: &BigUint) {
        // self.lp_token_supply().update(|x| *x -= amount);
        self.lp_token().nft_burn(nonce, amount)
    }

    fn get_lp_token_attributes<T: TopDecode>(&self, token_nonce: u64) -> T {
        self.lp_token().get_token_attributes(token_nonce)
    }

    #[view(getLpTokenId)]
    #[storage_mapper("lp_token_id")]
    fn lp_token(&self) -> NonFungibleTokenMapper;

    #[view(getLpTokenSupply)]
    #[storage_mapper("lp_token_supply")]
    fn lp_token_supply(&self) -> SingleValueMapper<BigUint>;
}
