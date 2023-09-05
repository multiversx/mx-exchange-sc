use multiversx_sc::codec::Empty;

multiversx_sc::imports!();

pub const LAUNCHED_TOKEN_REDEEM_NONCE: u64 = 1;
pub const ACCEPTED_TOKEN_REDEEM_NONCE: u64 = 2;

#[multiversx_sc::module]
pub trait RedeemTokenModule:
    crate::common_storage::CommonStorageModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueRedeemToken)]
    fn issue_redeem_token(
        &self,
        token_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        nr_decimals: usize,
    ) {
        let payment_amount = self.call_value().egld_value().clone_value();
        self.redeem_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            payment_amount,
            token_name,
            token_ticker,
            nr_decimals,
            None,
        );
    }

    #[only_owner]
    #[endpoint(createInitialRedeemTokens)]
    fn create_initial_redeem_tokens(&self) {
        require!(!self.redeem_token().is_empty(), "Token not issued");

        // create SFT for both types so NFTAddQuantity works
        let launched_token_id = self.launched_token_id().get();
        let accepted_token_id = self.accepted_token_id().get();
        let one = BigUint::from(1u32);

        let token_mapper = self.redeem_token();
        let _ = token_mapper.nft_create_named(
            one.clone(),
            launched_token_id.as_managed_buffer(),
            &Empty,
        );
        let _ = token_mapper.nft_create_named(one, &accepted_token_id.into_name(), &Empty);
    }

    fn mint_and_send_redeem_token(
        &self,
        to: &ManagedAddress,
        nonce: u64,
        amount: BigUint,
    ) -> EsdtTokenPayment<Self::Api> {
        self.redeem_token_total_circulating_supply(nonce)
            .update(|supply| *supply += &amount);

        self.redeem_token()
            .nft_add_quantity_and_send(to, nonce, amount)
    }

    fn burn_redeem_token(&self, nonce: u64, amount: &BigUint) {
        self.burn_redeem_token_without_supply_decrease(nonce, amount);

        self.redeem_token_total_circulating_supply(nonce)
            .update(|supply| *supply -= amount);
    }

    #[inline]
    fn burn_redeem_token_without_supply_decrease(&self, nonce: u64, amount: &BigUint) {
        self.redeem_token().nft_burn(nonce, amount);
    }

    #[view(getRedeemTokenId)]
    #[storage_mapper("redeemTokenId")]
    fn redeem_token(&self) -> NonFungibleTokenMapper;

    #[view(getRedeemTokenTotalCirculatingSupply)]
    #[storage_mapper("totalCirculatingSupply")]
    fn redeem_token_total_circulating_supply(&self, token_nonce: u64)
        -> SingleValueMapper<BigUint>;
}
