use common_structs::WrappedLpTokenAttributes;

use super::proxy_common;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::proxy_pair;
use factory::locked_asset_token_merge::ProxyTrait as _;
use proxy_pair::WrappedLpToken;

#[elrond_wasm::module]
pub trait WrappedLpTokenMerge:
    token_merge::TokenMergeModule + token_send::TokenSendModule + proxy_common::ProxyCommonModule
{
    #[payable("*")]
    #[endpoint(mergeWrappedLpTokens)]
    fn merge_wrapped_lp_tokens(&self) {
        let caller = self.blockchain().get_caller();
        let payments_vec = self.call_value().all_esdt_transfers();
        let payments_iter = payments_vec.iter();

        self.merge_wrapped_lp_tokens_and_send(&caller, payments_iter, Option::None);
    }

    fn merge_wrapped_lp_tokens_and_send(
        &self,
        caller: &ManagedAddress,
        payments: ManagedVecRefIterator<Self::Api, EsdtTokenPayment<Self::Api>>,
        replic: Option<WrappedLpToken<Self::Api>>,
    ) -> (WrappedLpToken<Self::Api>, bool) {
        require!(!payments.is_empty() || replic.is_some(), "Empty payments");
        let payments_len = payments.len();

        let wrapped_lp_token_id = self.wrapped_lp_token().get_token_id();
        self.require_all_tokens_are_wrapped_lp_tokens(payments.clone(), &wrapped_lp_token_id);

        let mut tokens = self.get_wrapped_lp_tokens_from_deposit(payments.clone());

        if replic.is_some() {
            tokens.push(replic.unwrap());
        }
        self.require_wrapped_lp_tokens_from_same_pair(&tokens);

        let merged_locked_token_amount = self.merge_locked_asset_tokens_from_wrapped_lp(&tokens);
        let merged_wrapped_lp_amount = self.get_merged_wrapped_lp_tokens_amount(&tokens);
        let lp_token_amount = EsdtTokenPayment::new(
            tokens.get(0).attributes.lp_token_id,
            0,
            merged_wrapped_lp_amount.clone(),
        );

        let attrs = self
            .get_merged_wrapped_lp_token_attributes(&lp_token_amount, &merged_locked_token_amount);
        self.burn_payment_tokens(payments);

        let new_nonce = self.send().esdt_nft_create_compact(
            &wrapped_lp_token_id,
            &merged_wrapped_lp_amount,
            &attrs,
        );

        self.send().direct_esdt(
            caller,
            &wrapped_lp_token_id,
            new_nonce,
            &merged_wrapped_lp_amount,
            &[],
        );

        let new_token = WrappedLpToken {
            token: EsdtTokenPayment::new(
                wrapped_lp_token_id,
                new_nonce,
                merged_wrapped_lp_amount,
            ),
            attributes: attrs,
        };
        let is_merged = payments_len != 0;

        (new_token, is_merged)
    }

    fn get_wrapped_lp_tokens_from_deposit(
        &self,
        payments: ManagedVecRefIterator<Self::Api, EsdtTokenPayment<Self::Api>>,
    ) -> ManagedVec<WrappedLpToken<Self::Api>> {
        let mut result = ManagedVec::new();

        for payment in payments {
            let attr = self
                .get_wrapped_lp_token_attributes(&payment.token_identifier, payment.token_nonce);

            result.push(WrappedLpToken {
                token: payment.clone(),
                attributes: attr,
            })
        }

        result
    }

    fn require_wrapped_lp_tokens_from_same_pair(
        &self,
        tokens: &ManagedVec<WrappedLpToken<Self::Api>>,
    ) {
        let lp_token_id = tokens.get(0).attributes.lp_token_id;

        for elem in tokens.iter() {
            require!(
                elem.attributes.lp_token_id == lp_token_id,
                "Lp token id differs"
            );
        }
    }

    fn require_all_tokens_are_wrapped_lp_tokens(
        &self,
        tokens: ManagedVecRefIterator<Self::Api, EsdtTokenPayment<Self::Api>>,
        wrapped_lp_token_id: &TokenIdentifier,
    ) {
        for elem in tokens {
            require!(
                &elem.token_identifier == wrapped_lp_token_id,
                "Not a Wrapped Lp Token"
            );
        }
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
        tokens: &ManagedVec<WrappedLpToken<Self::Api>>,
    ) -> EsdtTokenPayment<Self::Api> {
        let locked_asset_factory_addr = self.locked_asset_factory_address().get();
        let locked_asset_token = self.locked_asset_token_id().get();

        if tokens.len() == 1 {
            let token = tokens.get(0);

            let amount = self.rule_of_three_non_zero_result(
                &token.token.amount,
                &token.attributes.lp_token_total_amount,
                &token.attributes.locked_assets_invested,
            );

            return EsdtTokenPayment::new(
                locked_asset_token,
                token.attributes.locked_assets_nonce,
                amount,
            );
        }

        let mut payments = ManagedVec::new();
        for entry in tokens.iter() {
            let amount = self.rule_of_three_non_zero_result(
                &entry.token.amount,
                &entry.attributes.lp_token_total_amount,
                &entry.attributes.locked_assets_invested,
            );

            payments.push(EsdtTokenPayment::new(
                locked_asset_token.clone(),
                entry.attributes.locked_assets_nonce,
                amount,
            ));
        }

        self.locked_asset_factory_proxy(locked_asset_factory_addr)
            .merge_locked_asset_tokens()
            .with_multi_token_transfer(payments)
            .execute_on_dest_context()
    }

    fn get_merged_wrapped_lp_tokens_amount(
        &self,
        tokens: &ManagedVec<WrappedLpToken<Self::Api>>,
    ) -> BigUint {
        let mut token_amount = BigUint::zero();

        tokens
            .iter()
            .for_each(|x| token_amount += &x.token.amount);
        token_amount
    }
}
