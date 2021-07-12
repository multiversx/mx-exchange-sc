use common_structs::{GenericTokenAmountPair, WrappedLpTokenAttributes};

use super::proxy_common;
use proxy_common::ACCEPT_PAY_FUNC_NAME;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::proxy_pair;
use nft_deposit::ProxyTrait as _;
use proxy_pair::WrappedLpToken;
use sc_locked_asset_factory::locked_asset_token_merge::ProxyTrait as _;

#[elrond_wasm_derive::module]
pub trait WrappedLpTokenMerge:
    token_merge::TokenMergeModule
    + token_send::TokenSendModule
    + token_supply::TokenSupplyModule
    + proxy_common::ProxyCommonModule
    + nft_deposit::NftDepositModule
{
    #[proxy]
    fn locked_asset_factory(&self, to: Address) -> sc_locked_asset_factory::Proxy<Self::SendApi>;

    #[endpoint(mergeWrappedLpTokens)]
    fn merge_wrapped_lp_tokens(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        self.merge_wrapped_lp_tokens_and_send(&caller, Option::None, opt_accept_funds_func)
    }

    fn merge_wrapped_lp_tokens_and_send(
        &self,
        caller: &Address,
        replic: Option<WrappedLpToken<Self::BigUint>>,
        opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let deposit = self.nft_deposit(caller).get();
        require!(!deposit.is_empty() || replic.is_some(), "Empty deposit");

        let wrapped_lp_token_id = self.wrapped_lp_token_id().get();
        self.require_all_tokens_are_wrapped_lp_tokens(&deposit, &wrapped_lp_token_id)?;

        let mut tokens = self.get_wrapped_lp_tokens_from_deposit(&deposit)?;

        if replic.is_some() {
            tokens.push(replic.unwrap());
        }
        self.require_wrapped_lp_tokens_from_same_pair(&tokens)?;

        let merged_locked_token_amount = self.merge_locked_asset_tokens_from_wrapped_lp(&tokens);
        let attrs =
            self.get_merged_wrapped_lp_token_attributes(&tokens, &merged_locked_token_amount);
        let amount = self.get_merged_wrapped_lp_tokens_amount(&tokens);
        self.burn_deposit_tokens(caller, &deposit);

        self.nft_create_tokens(&wrapped_lp_token_id, &amount, &attrs);
        let new_nonce = self.increase_wrapped_lp_token_nonce();

        self.send_nft_tokens(
            &wrapped_lp_token_id,
            new_nonce,
            &amount,
            caller,
            &opt_accept_funds_func,
        );

        Ok(())
    }
    fn get_wrapped_lp_tokens_from_deposit(
        &self,
        deposit: &[GenericTokenAmountPair<Self::BigUint>],
    ) -> SCResult<Vec<WrappedLpToken<Self::BigUint>>> {
        let mut result = Vec::new();

        for elem in deposit.iter() {
            result.push(WrappedLpToken {
                token_amount: elem.clone(),
                attributes: self
                    .get_wrapped_lp_token_attributes(&elem.token_id, elem.token_nonce)?,
            })
        }
        Ok(result)
    }

    fn require_wrapped_lp_tokens_from_same_pair(
        &self,
        tokens: &[WrappedLpToken<Self::BigUint>],
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
        tokens: &[GenericTokenAmountPair<Self::BigUint>],
        wrapped_lp_token_id: &TokenIdentifier,
    ) -> SCResult<()> {
        for elem in tokens.iter() {
            require!(
                &elem.token_id == wrapped_lp_token_id,
                "Not a Wrapped Lp Token"
            );
        }
        Ok(())
    }

    fn get_merged_wrapped_lp_token_attributes(
        &self,
        tokens: &[WrappedLpToken<Self::BigUint>],
        merged_locked_asset_token_amount: &GenericTokenAmountPair<Self::BigUint>,
    ) -> WrappedLpTokenAttributes<Self::BigUint> {
        let mut lp_token_amount = Self::BigUint::zero();

        tokens
            .iter()
            .for_each(|x| lp_token_amount += &x.attributes.lp_token_total_amount);
        WrappedLpTokenAttributes {
            lp_token_id: tokens[0].attributes.lp_token_id.clone(),
            lp_token_total_amount: lp_token_amount,
            locked_assets_invested: merged_locked_asset_token_amount.amount.clone(),
            locked_assets_nonce: merged_locked_asset_token_amount.token_nonce,
        }
    }

    fn merge_locked_asset_tokens_from_wrapped_lp(
        &self,
        tokens: &[WrappedLpToken<Self::BigUint>],
    ) -> GenericTokenAmountPair<Self::BigUint> {
        let locked_asset_factory_addr = self.locked_asset_factory_address().get();
        let locked_asset_token = self.locked_asset_token_id().get();

        if tokens.len() == 1 {
            let token = tokens[0].clone();

            let amount = self.rule_of_three(
                &token.token_amount.amount,
                &token.attributes.lp_token_total_amount,
                &token.attributes.locked_assets_invested,
            );
            return GenericTokenAmountPair {
                token_id: locked_asset_token,
                token_nonce: token.attributes.locked_assets_nonce,
                amount,
            };
        }

        for entry in tokens.iter() {
            let amount = self.rule_of_three(
                &entry.token_amount.amount,
                &entry.attributes.lp_token_total_amount,
                &entry.attributes.locked_assets_invested,
            );

            self.locked_asset_factory(locked_asset_factory_addr.clone())
                .deposit_tokens(
                    locked_asset_token.clone(),
                    entry.attributes.locked_assets_nonce,
                    amount,
                )
                .execute_on_dest_context();
        }

        self.locked_asset_factory(locked_asset_factory_addr)
            .merge_locked_asset_tokens(OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)))
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after))
    }

    fn get_merged_wrapped_lp_tokens_amount(
        &self,
        tokens: &[WrappedLpToken<Self::BigUint>],
    ) -> Self::BigUint {
        let mut token_amount = Self::BigUint::zero();

        tokens
            .iter()
            .for_each(|x| token_amount += &x.token_amount.amount);
        token_amount
    }
}
