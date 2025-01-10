multiversx_sc::imports!();

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
            payment_amount,
            token_name,
            token_ticker,
            nr_decimals,
            None,
        );
    }

    fn mint_and_send_redeem_token(&self, to: &ManagedAddress, amount: BigUint) -> EsdtTokenPayment {
        self.redeem_token_total_circulating_supply()
            .update(|supply| *supply += &amount);

        self.redeem_token().mint_and_send(to, amount)
    }

    fn burn_redeem_token(&self, amount: &BigUint) {
        self.burn_redeem_token_without_supply_decrease(amount);

        self.redeem_token_total_circulating_supply()
            .update(|supply| *supply -= amount);
    }

    #[inline]
    fn burn_redeem_token_without_supply_decrease(&self, amount: &BigUint) {
        self.redeem_token().burn(amount);
    }

    #[view(getRedeemTokenId)]
    #[storage_mapper("redeemTokenId")]
    fn redeem_token(&self) -> FungibleTokenMapper;

    #[view(getRedeemTokenTotalCirculatingSupply)]
    #[storage_mapper("totalCirculatingSupply")]
    fn redeem_token_total_circulating_supply(&self) -> SingleValueMapper<BigUint>;

    // TODO: Save launched token balance somewhere
}
