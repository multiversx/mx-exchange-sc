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

    #[only_owner]
    #[endpoint(setTransferRole)]
    fn set_transfer_role(&self) {
        self.redeem_token().set_local_roles(
            &[EsdtLocalRole::Transfer],
            Some(<Self as RedeemTokenModule>::callbacks(self).set_transfer_role_callback()),
        );
    }

    #[callback]
    fn set_transfer_role_callback(&self, #[call_result] result: ManagedAsyncCallResult<()>) {
        match result {
            ManagedAsyncCallResult::Ok(()) => {
                self.transfer_role_set().set(true);
            }
            ManagedAsyncCallResult::Err(_) => {
                sc_panic!("Failed setting transfer role");
            }
        }
    }

    fn require_redeem_token_issued(&self) {
        require!(!self.redeem_token().is_empty(), "Redeem token not issued");
    }

    fn require_redeem_token_transfer_role_set(&self) {
        require!(
            self.transfer_role_set().get(),
            "Redeem token transfer role not set"
        );
    }

    fn require_redeem_token_setup_complete(&self) {
        self.require_redeem_token_issued();
        self.require_redeem_token_transfer_role_set();
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

    #[storage_mapper("transferRoleSet")]
    fn transfer_role_set(&self) -> SingleValueMapper<bool>;

    #[view(getRedeemTokenTotalCirculatingSupply)]
    #[storage_mapper("totalCirculatingSupply")]
    fn redeem_token_total_circulating_supply(&self) -> SingleValueMapper<BigUint>;
}
