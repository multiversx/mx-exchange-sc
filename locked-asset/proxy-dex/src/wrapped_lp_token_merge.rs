use common_structs::WrappedLpTokenAttributes;

use super::proxy_common;
use proxy_common::ACCEPT_PAY_FUNC_NAME;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::proxy_pair;
use factory::locked_asset_token_merge::ProxyTrait as _;
use proxy_pair::WrappedLpToken;

#[elrond_wasm::module]
pub trait WrappedLpTokenMerge:
    token_merge::TokenMergeModule
    + token_send::TokenSendModule
    + token_supply::TokenSupplyModule
    + proxy_common::ProxyCommonModule
{
    #[proxy]
    fn locked_asset_factory(&self, to: ManagedAddress) -> factory::Proxy<Self::Api>;

    #[payable("*")]
    #[endpoint(mergeWrappedLpTokens)]
    fn merge_wrapped_lp_tokens(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let payments = self.get_all_payments();

        self.merge_wrapped_lp_tokens_and_send(
            &caller,
            &payments,
            Option::None,
            opt_accept_funds_func,
        )?;
        Ok(())
    }

    fn merge_wrapped_lp_tokens_and_send(
        &self,
        caller: &ManagedAddress,
        payments: &[EsdtTokenPayment<Self::Api>],
        replic: Option<WrappedLpToken<Self::Api>>,
        opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<(WrappedLpToken<Self::Api>, bool)> {
        require!(!payments.is_empty() || replic.is_some(), "Empty payments");
        let payments_len = payments.len();

        let wrapped_lp_token_id = self.wrapped_lp_token_id().get();
        self.require_all_tokens_are_wrapped_lp_tokens(payments, &wrapped_lp_token_id)?;

        let mut tokens = self.get_wrapped_lp_tokens_from_deposit(payments);

        if replic.is_some() {
            tokens.push(replic.unwrap());
        }
        self.require_wrapped_lp_tokens_from_same_pair(&tokens)?;

        let merged_locked_token_amount = self.merge_locked_asset_tokens_from_wrapped_lp(&tokens)?;
        let merged_wrapped_lp_amount = self.get_merged_wrapped_lp_tokens_amount(&tokens);
        let lp_token_amount = self.create_payment(
            &tokens[0].attributes.lp_token_id,
            0,
            &merged_wrapped_lp_amount,
        );

        let attrs = self
            .get_merged_wrapped_lp_token_attributes(&lp_token_amount, &merged_locked_token_amount);
        self.burn_payment_tokens(payments);

        let new_nonce =
            self.nft_create_tokens(&wrapped_lp_token_id, &merged_wrapped_lp_amount, &attrs);

        self.transfer_execute_custom(
            caller,
            &wrapped_lp_token_id,
            new_nonce,
            &merged_wrapped_lp_amount,
            &opt_accept_funds_func,
        )?;

        let new_token = WrappedLpToken {
            token_amount: self.create_payment(
                &wrapped_lp_token_id,
                new_nonce,
                &merged_wrapped_lp_amount,
            ),
            attributes: attrs,
        };
        let is_merged = payments_len != 0;

        Ok((new_token, is_merged))
    }

    fn get_wrapped_lp_tokens_from_deposit(
        &self,
        payments: &[EsdtTokenPayment<Self::Api>],
    ) -> Vec<WrappedLpToken<Self::Api>> {
        let mut result = Vec::new();

        for payment in payments.iter() {
            result.push(WrappedLpToken {
                token_amount: payment.clone(),
                attributes: self.get_wrapped_lp_token_attributes(
                    &payment.token_identifier,
                    payment.token_nonce,
                ),
            })
        }
        result
    }

    fn require_wrapped_lp_tokens_from_same_pair(
        &self,
        tokens: &[WrappedLpToken<Self::Api>],
    ) -> SCResult<()> {
        let lp_token_id = tokens[0].attributes.lp_token_id.clone();

        for elem in tokens.iter() {
            require!(
                elem.attributes.lp_token_id == lp_token_id,
                "Lp token id differs"
            );
        }
        Ok(())
    }

    fn require_all_tokens_are_wrapped_lp_tokens(
        &self,
        tokens: &[EsdtTokenPayment<Self::Api>],
        wrapped_lp_token_id: &TokenIdentifier,
    ) -> SCResult<()> {
        for elem in tokens.iter() {
            require!(
                &elem.token_identifier == wrapped_lp_token_id,
                "Not a Wrapped Lp Token"
            );
        }
        Ok(())
    }

    fn get_merged_wrapped_lp_token_attributes(
        &self,
        lp_token_amount: &EsdtTokenPayment<Self::Api>,
        merged_locked_asset_token_amount: &EsdtTokenPayment<Self::Api>,
    ) -> WrappedLpTokenAttributes<Self::Api> {
        WrappedLpTokenAttributes {
            lp_token_id: lp_token_amount.token_identifier.clone(),
            lp_token_total_amount: lp_token_amount.amount.clone(),
            locked_assets_invested: merged_locked_asset_token_amount.amount.clone(),
            locked_assets_nonce: merged_locked_asset_token_amount.token_nonce,
        }
    }

    fn merge_locked_asset_tokens_from_wrapped_lp(
        &self,
        tokens: &[WrappedLpToken<Self::Api>],
    ) -> SCResult<EsdtTokenPayment<Self::Api>> {
        let locked_asset_factory_addr = self.locked_asset_factory_address().get();
        let locked_asset_token = self.locked_asset_token_id().get();

        if tokens.len() == 1 {
            let token = tokens[0].clone();

            let amount = self.rule_of_three_non_zero_result(
                &token.token_amount.amount,
                &token.attributes.lp_token_total_amount,
                &token.attributes.locked_assets_invested,
            )?;

            return Ok(self.create_payment(
                &locked_asset_token,
                token.attributes.locked_assets_nonce,
                &amount,
            ));
        }

        let mut payments = ManagedVec::new();
        for entry in tokens.iter() {
            let amount = self.rule_of_three_non_zero_result(
                &entry.token_amount.amount,
                &entry.attributes.lp_token_total_amount,
                &entry.attributes.locked_assets_invested,
            )?;

            payments.push(EsdtTokenPayment::from(
                locked_asset_token.clone(),
                entry.attributes.locked_assets_nonce,
                amount,
            ));
        }

        Ok(self
            .locked_asset_factory(locked_asset_factory_addr)
            .merge_locked_asset_tokens(OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)))
            .with_multi_token_transfer(payments)
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after)))
    }

    fn get_merged_wrapped_lp_tokens_amount(&self, tokens: &[WrappedLpToken<Self::Api>]) -> BigUint {
        let mut token_amount = BigUint::zero();

        tokens
            .iter()
            .for_each(|x| token_amount += &x.token_amount.amount);
        token_amount
    }
}
