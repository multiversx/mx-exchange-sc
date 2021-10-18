use common_structs::{FftTokenAmountPair, GenericTokenAmountPair, WrappedFarmTokenAttributes};

use super::proxy_common;
use proxy_common::ACCEPT_PAY_FUNC_NAME;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::wrapped_lp_token_merge;

use super::proxy_pair;
use proxy_pair::WrappedLpToken;

use super::proxy_farm;
use proxy_farm::WrappedFarmToken;

use elrond_dex_farm::farm_token_merge::ProxyTrait as _;
use sc_locked_asset_factory::locked_asset_token_merge::ProxyTrait as _;

#[elrond_wasm::module]
pub trait WrappedFarmTokenMerge:
    token_merge::TokenMergeModule
    + token_send::TokenSendModule
    + token_supply::TokenSupplyModule
    + proxy_common::ProxyCommonModule
    + wrapped_lp_token_merge::WrappedLpTokenMerge
{
    #[proxy]
    fn locked_asset_factory_proxy(
        &self,
        to: ManagedAddress,
    ) -> sc_locked_asset_factory::Proxy<Self::Api>;

    #[proxy]
    fn farm_contract_merge_proxy(&self, to: ManagedAddress) -> elrond_dex_farm::Proxy<Self::Api>;

    #[payable("*")]
    #[endpoint(mergeWrappedFarmTokens)]
    fn merge_wrapped_farm_tokens(
        &self,
        farm_contract: ManagedAddress,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        require!(
            self.intermediated_farms().contains(&farm_contract),
            "Invalid farm contract address"
        );
        let payments = self.get_all_payments();

        self.merge_wrapped_farm_tokens_and_send(
            &caller,
            &farm_contract,
            &payments,
            Option::None,
            opt_accept_funds_func,
        )?;
        Ok(())
    }

    fn merge_wrapped_farm_tokens_and_send(
        &self,
        caller: &ManagedAddress,
        farm_contract: &ManagedAddress,
        payments: &[EsdtTokenPayment<Self::Api>],
        replic: Option<WrappedFarmToken<Self::Api>>,
        opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<(WrappedFarmToken<Self::Api>, bool)> {
        require!(!payments.is_empty() || replic.is_some(), "Empty deposit");
        let deposit_len = payments.len();

        let wrapped_farm_token_id = self.wrapped_farm_token_id().get();
        self.require_all_tokens_are_wrapped_farm_tokens(payments, &wrapped_farm_token_id)?;

        let mut tokens = self.get_wrapped_farm_tokens_from_deposit(payments)?;

        if replic.is_some() {
            tokens.push(replic.unwrap());
        }
        self.require_wrapped_farm_tokens_from_same_farm(&tokens)?;

        let merged_farm_token_amount = self.merge_farm_tokens(farm_contract, &tokens);
        let farming_token_amount = self.merge_farming_tokens(&tokens)?;
        self.burn_payment_tokens(payments);

        let new_attrs = WrappedFarmTokenAttributes {
            farm_token_id: merged_farm_token_amount.token_id.clone(),
            farm_token_nonce: merged_farm_token_amount.token_nonce,
            farm_token_amount: merged_farm_token_amount.amount.clone(),
            farming_token_id: farming_token_amount.token_id,
            farming_token_nonce: farming_token_amount.token_nonce,
            farming_token_amount: farming_token_amount.amount,
        };

        self.nft_create_tokens(
            &wrapped_farm_token_id,
            &merged_farm_token_amount.amount,
            &new_attrs,
        );
        let new_nonce = self.increase_wrapped_farm_token_nonce();

        self.direct_esdt_nft_execute_custom(
            caller,
            &wrapped_farm_token_id,
            new_nonce,
            &merged_farm_token_amount.amount,
            &opt_accept_funds_func,
        )?;

        let new_token = WrappedFarmToken {
            token_amount: merged_farm_token_amount,
            attributes: new_attrs,
        };
        let is_merged = deposit_len != 0;

        Ok((new_token, is_merged))
    }

    fn get_wrapped_farm_tokens_from_deposit(
        &self,
        payments: &[EsdtTokenPayment<Self::Api>],
    ) -> SCResult<Vec<WrappedFarmToken<Self::Api>>> {
        let mut result = Vec::new();

        for payment in payments.iter() {
            result.push(WrappedFarmToken {
                token_amount: GenericTokenAmountPair {
                    token_id: payment.token_identifier.clone(),
                    token_nonce: payment.token_nonce,
                    amount: payment.amount.clone(),
                },
                attributes: self.get_wrapped_farm_token_attributes(
                    &payment.token_identifier,
                    payment.token_nonce,
                )?,
            })
        }
        Ok(result)
    }

    fn require_wrapped_farm_tokens_from_same_farm(
        &self,
        tokens: &[WrappedFarmToken<Self::Api>],
    ) -> SCResult<()> {
        let farm_token_id = tokens[0].attributes.farm_token_id.clone();

        for elem in tokens.iter() {
            require!(
                elem.attributes.farm_token_id == farm_token_id,
                "Farm token id differs"
            );
        }
        Ok(())
    }

    fn require_all_tokens_are_wrapped_farm_tokens(
        &self,
        tokens: &[EsdtTokenPayment<Self::Api>],
        wrapped_farm_token_id: &TokenIdentifier,
    ) -> SCResult<()> {
        for elem in tokens.iter() {
            require!(
                &elem.token_identifier == wrapped_farm_token_id,
                "Not a Wrapped Farm Token"
            );
        }
        Ok(())
    }

    fn merge_locked_asset_tokens_from_wrapped_farm(
        &self,
        tokens: &[WrappedFarmToken<Self::Api>],
    ) -> SCResult<GenericTokenAmountPair<Self::Api>> {
        let locked_asset_factory_addr = self.locked_asset_factory_address().get();

        if tokens.len() == 1 {
            let token = tokens[0].clone();
            let locked_token_amount = self.rule_of_three_non_zero_result(
                &token.token_amount.amount,
                &token.attributes.farm_token_amount,
                &token.attributes.farming_token_amount,
            )?;

            return Ok(GenericTokenAmountPair {
                token_id: self.locked_asset_token_id().get(),
                token_nonce: token.attributes.farming_token_nonce,
                amount: locked_token_amount,
            });
        }

        let locked_asset_token = self.locked_asset_token_id().get();
        let mut payments = ManagedVec::new();
        for entry in tokens.iter() {
            let locked_token_amount = self.rule_of_three_non_zero_result(
                &entry.token_amount.amount,
                &entry.attributes.farm_token_amount,
                &entry.attributes.farming_token_amount,
            )?;

            payments.push(EsdtTokenPayment::from(
                locked_asset_token.clone(),
                entry.attributes.farming_token_nonce,
                locked_token_amount,
            ));
        }

        Ok(self
            .locked_asset_factory_proxy(locked_asset_factory_addr)
            .merge_locked_asset_tokens(OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)))
            .with_multi_token_transfer(payments)
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after)))
    }

    fn merge_farm_tokens(
        &self,
        farm_contract: &ManagedAddress,
        tokens: &[WrappedFarmToken<Self::Api>],
    ) -> GenericTokenAmountPair<Self::Api> {
        if tokens.len() == 1 {
            let token = tokens[0].clone();

            return GenericTokenAmountPair {
                token_id: token.attributes.farm_token_id,
                token_nonce: token.attributes.farm_token_nonce,
                amount: token.token_amount.amount,
            };
        }

        let mut payments = ManagedVec::new();
        for entry in tokens.iter() {
            payments.push(EsdtTokenPayment::from(
                entry.attributes.farm_token_id.clone(),
                entry.attributes.farm_token_nonce,
                entry.token_amount.amount.clone(),
            ));
        }

        self.farm_contract_merge_proxy(farm_contract.clone())
            .merge_farm_tokens(OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)))
            .with_multi_token_transfer(payments)
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after))
    }

    fn merge_farming_tokens(
        &self,
        tokens: &[WrappedFarmToken<Self::Api>],
    ) -> SCResult<GenericTokenAmountPair<Self::Api>> {
        if tokens.len() == 1 {
            let first_token = tokens[0].clone();
            let farming_amount = self.rule_of_three_non_zero_result(
                &first_token.token_amount.amount,
                &first_token.attributes.farm_token_amount,
                &first_token.attributes.farming_token_amount,
            )?;

            return Ok(GenericTokenAmountPair {
                token_id: first_token.attributes.farming_token_id,
                token_nonce: first_token.attributes.farming_token_nonce,
                amount: farming_amount,
            });
        }

        let farming_token_id = tokens[0].clone().attributes.farming_token_id;
        let locked_asset_token_id = self.locked_asset_token_id().get();

        if farming_token_id == locked_asset_token_id {
            self.merge_locked_asset_tokens_from_wrapped_farm(tokens)
        } else {
            self.merge_wrapped_lp_tokens_from_farm(tokens)
        }
    }

    fn merge_wrapped_lp_tokens_from_farm(
        &self,
        tokens: &[WrappedFarmToken<Self::Api>],
    ) -> SCResult<GenericTokenAmountPair<Self::Api>> {
        let mut wrapped_lp_tokens = Vec::new();

        for token in tokens.iter() {
            let wrapped_lp_token_amount = self.rule_of_three_non_zero_result(
                &token.token_amount.amount,
                &token.attributes.farm_token_amount,
                &token.attributes.farming_token_amount,
            )?;

            let wrapped_lp_token_id = token.attributes.farming_token_id.clone();
            let wrapped_lp_token_nonce = token.attributes.farming_token_nonce;

            let attributes =
                self.get_wrapped_lp_token_attributes(&wrapped_lp_token_id, wrapped_lp_token_nonce)?;
            let wrapped_lp_token = WrappedLpToken {
                token_amount: GenericTokenAmountPair {
                    token_id: wrapped_lp_token_id.clone(),
                    token_nonce: wrapped_lp_token_nonce,
                    amount: wrapped_lp_token_amount,
                },
                attributes,
            };
            wrapped_lp_tokens.push(wrapped_lp_token);
        }

        self.require_wrapped_lp_tokens_from_same_pair(&wrapped_lp_tokens)?;
        let merged_locked_token_amount =
            self.merge_locked_asset_tokens_from_wrapped_lp(&wrapped_lp_tokens)?;
        let merged_wrapped_lp_token_amount =
            self.get_merged_wrapped_lp_tokens_amount(&wrapped_lp_tokens);
        let lp_token_amount = FftTokenAmountPair {
            token_id: wrapped_lp_tokens[0].attributes.lp_token_id.clone(),
            amount: merged_wrapped_lp_token_amount.clone(),
        };

        let attrs = self
            .get_merged_wrapped_lp_token_attributes(&lp_token_amount, &merged_locked_token_amount);

        let wrapped_lp_token_id = tokens[0].attributes.farming_token_id.clone();
        self.nft_create_tokens(
            &wrapped_lp_token_id,
            &merged_wrapped_lp_token_amount,
            &attrs,
        );
        let new_nonce = self.increase_wrapped_lp_token_nonce();

        for wrapped_lp_token in wrapped_lp_tokens.iter() {
            self.nft_burn_tokens(
                &wrapped_lp_token.token_amount.token_id,
                wrapped_lp_token.token_amount.token_nonce,
                &wrapped_lp_token.token_amount.amount,
            );
        }

        Ok(GenericTokenAmountPair {
            token_id: wrapped_lp_token_id,
            token_nonce: new_nonce,
            amount: merged_wrapped_lp_token_amount,
        })
    }
}
