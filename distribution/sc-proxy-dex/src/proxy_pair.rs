#![allow(clippy::too_many_arguments)]
#![allow(clippy::comparison_chain)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use core::iter::FromIterator;

use proxy_common::ACCEPT_PAY_FUNC_NAME;
const MAX_USER_TEMPORARY_SIZE: usize = 10;

use common_structs::{FftTokenAmountPair, GenericTokenAmountPair, Nonce, WrappedLpTokenAttributes};

use super::wrapped_lp_token_merge;

use super::proxy_common;

type AddLiquidityResultType<BigUint> = MultiResult3<
    FftTokenAmountPair<BigUint>,
    FftTokenAmountPair<BigUint>,
    FftTokenAmountPair<BigUint>,
>;

type RemoveLiquidityResultType<BigUint> =
    MultiResult2<FftTokenAmountPair<BigUint>, FftTokenAmountPair<BigUint>>;

#[derive(Clone)]
pub struct WrappedLpToken<BigUint: BigUintApi> {
    pub token_amount: GenericTokenAmountPair<BigUint>,
    pub attributes: WrappedLpTokenAttributes<BigUint>,
}

#[elrond_wasm_derive::module]
pub trait ProxyPairModule:
    proxy_common::ProxyCommonModule
    + token_supply::TokenSupplyModule
    + wrapped_lp_token_merge::WrappedLpTokenMerge
    + token_merge::TokenMergeModule
    + token_send::TokenSendModule
    + nft_deposit::NftDepositModule
{
    #[proxy]
    fn pair_contract_proxy(&self, to: Address) -> elrond_dex_pair::Proxy<Self::SendApi>;

    #[endpoint(addPairToIntermediate)]
    fn add_pair_to_intermediate(&self, pair_address: Address) -> SCResult<()> {
        self.require_permissions()?;
        self.intermediated_pairs().insert(pair_address);
        Ok(())
    }

    #[endpoint(removeIntermediatedPair)]
    fn remove_intermediated_pair(&self, pair_address: Address) -> SCResult<()> {
        self.require_permissions()?;
        self.require_is_intermediated_pair(&pair_address)?;
        self.intermediated_pairs().remove(&pair_address);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(acceptEsdtPaymentProxy)]
    fn accept_esdt_payment_proxy(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
        #[payment_nonce] token_nonce: Nonce,
        pair_address: Address,
    ) -> SCResult<()> {
        self.require_is_intermediated_pair(&pair_address)?;
        require!(amount != 0, "Payment amount cannot be zero");
        require!(
            token_id == self.locked_asset_token_id().get() || token_nonce == 0,
            "Bad input token"
        );

        let caller = self.blockchain().get_caller();
        require!(
            self.temporary_funds(&caller).len() <= MAX_USER_TEMPORARY_SIZE,
            "Temporary funds storage for this address exceeded maximum"
        );

        self.increase_temporary_funds_amount(&caller, &token_id, token_nonce, &amount);
        Ok(())
    }

    #[endpoint(reclaimTemporaryFundsProxy)]
    fn reclaim_temporary_funds_proxy(
        &self,
        first_token_id: TokenIdentifier,
        first_token_nonce: Nonce,
        second_token_id: TokenIdentifier,
        second_token_nonce: Nonce,
    ) -> SCResult<()> {
        require!(
            first_token_id != second_token_id || first_token_nonce != second_token_nonce,
            "Identical tokens"
        );
        let caller = self.blockchain().get_caller();
        let first_token_amount = self
            .temporary_funds(&caller)
            .get(&(first_token_id.clone(), first_token_nonce))
            .unwrap_or_else(Self::BigUint::zero);
        let second_token_amount = self
            .temporary_funds(&caller)
            .get(&(second_token_id.clone(), second_token_nonce))
            .unwrap_or_else(Self::BigUint::zero);
        self.temporary_funds(&caller)
            .remove(&(first_token_id.clone(), first_token_nonce));
        self.temporary_funds(&caller)
            .remove(&(second_token_id.clone(), second_token_nonce));
        self.direct_generic_safe(
            &caller,
            &first_token_id,
            first_token_nonce,
            &first_token_amount,
        );
        self.direct_generic_safe(
            &caller,
            &second_token_id,
            second_token_nonce,
            &second_token_amount,
        );
        Ok(())
    }

    #[endpoint(addLiquidityProxy)]
    fn add_liquidity_proxy(
        &self,
        pair_address: Address,
        first_token_id: TokenIdentifier,
        first_token_nonce: Nonce,
        first_token_amount_desired: Self::BigUint,
        first_token_amount_min: Self::BigUint,
        second_token_id: TokenIdentifier,
        second_token_nonce: Nonce,
        second_token_amount_desired: Self::BigUint,
        second_token_amount_min: Self::BigUint,
    ) -> SCResult<()> {
        self.require_is_intermediated_pair(&pair_address)?;
        self.require_wrapped_lp_token_id_not_empty()?;

        let caller = self.blockchain().get_caller();
        require!(first_token_id != second_token_id, "Identical tokens");
        require!(
            (first_token_nonce == 0 && second_token_nonce != 0)
                || (first_token_nonce != 0 && second_token_nonce == 0),
            "This endpoint accepts one Fungible and one SemiFungible"
        );
        require!(
            first_token_amount_desired > 0 && second_token_amount_desired > 0,
            "Cannot add zero amount"
        );
        let locked_asset_token_id = self.locked_asset_token_id().get();
        require!(
            (first_token_nonce != 0 && first_token_id == locked_asset_token_id)
                || (second_token_nonce != 0 && second_token_id == locked_asset_token_id),
            "The SemiFungible token should be the locked asset"
        );
        let first_token_amount_temporary = self
            .temporary_funds(&caller)
            .get(&(first_token_id.clone(), first_token_nonce))
            .unwrap_or_else(Self::BigUint::zero);
        require!(
            first_token_amount_temporary >= first_token_amount_desired,
            "Not enough first temporary funds"
        );
        let second_token_amount_temporary = self
            .temporary_funds(&caller)
            .get(&(second_token_id.clone(), second_token_nonce))
            .unwrap_or_else(Self::BigUint::zero);
        require!(
            second_token_amount_temporary >= second_token_amount_desired,
            "Not enough second temporary funds"
        );

        // Actual 2x acceptEsdtPayment
        self.forward_to_pair(
            &pair_address,
            &first_token_id,
            first_token_nonce,
            &first_token_amount_desired,
        );
        self.forward_to_pair(
            &pair_address,
            &second_token_id,
            second_token_nonce,
            &second_token_amount_desired,
        );

        // Actual adding of liquidity
        self.reset_received_funds_on_current_tx();
        let result = self.actual_add_liquidity(
            &pair_address,
            &first_token_amount_desired,
            &first_token_amount_min,
            &second_token_amount_desired,
            &second_token_amount_min,
        );

        let result_tuple = result.0;
        let lp_received = result_tuple.0;
        let first_token_used = result_tuple.1;
        let second_token_used = result_tuple.2;
        require!(
            lp_received.amount > 0,
            "LP token amount should be greater than 0"
        );
        require!(
            first_token_used.token_id == first_token_id
                || second_token_used.token_id == second_token_id,
            "Bad token order"
        );
        require!(
            first_token_used.amount <= first_token_amount_desired
                && second_token_used.amount <= second_token_amount_desired,
            "Used more tokens than provided"
        );
        self.validate_received_funds_chunk(
            [
                (&lp_received.token_id, 0, &lp_received.amount),
                (
                    &first_token_used.token_id,
                    0,
                    &(&first_token_amount_desired - &first_token_used.amount),
                ),
                (
                    &second_token_used.token_id,
                    0,
                    &(&second_token_amount_desired - &second_token_used.amount),
                ),
            ]
            .to_vec(),
        )?;

        //Recalculate temporary funds and burn unused
        let locked_asset_token_nonce: Nonce;
        let consumed_locked_tokens: Self::BigUint;
        let asset_token_id = self.asset_token_id().get();
        let unused_minted_assets: Self::BigUint;
        if first_token_used.token_id == asset_token_id {
            consumed_locked_tokens = first_token_used.amount;
            unused_minted_assets = first_token_amount_desired - consumed_locked_tokens.clone();
            locked_asset_token_nonce = first_token_nonce;

            self.decrease_temporary_funds_amount(
                &caller,
                &first_token_id,
                first_token_nonce,
                &consumed_locked_tokens,
            );
            self.decrease_temporary_funds_amount(
                &caller,
                &second_token_used.token_id,
                second_token_nonce,
                &second_token_used.amount,
            );
        } else if second_token_used.token_id == asset_token_id {
            consumed_locked_tokens = second_token_used.amount;
            unused_minted_assets = second_token_amount_desired - consumed_locked_tokens.clone();
            locked_asset_token_nonce = second_token_nonce;

            self.decrease_temporary_funds_amount(
                &caller,
                &first_token_used.token_id,
                first_token_nonce,
                &first_token_used.amount,
            );
            self.decrease_temporary_funds_amount(
                &caller,
                &second_token_id,
                second_token_nonce,
                &consumed_locked_tokens,
            );
        } else {
            return sc_error!("Add liquidity did not return asset token id");
        }

        self.reclaim_temporary_funds_proxy(
            first_token_id,
            first_token_nonce,
            second_token_id,
            second_token_nonce,
        )?;
        self.create_by_merging_and_send(
            &lp_received.token_id,
            &lp_received.amount,
            &consumed_locked_tokens,
            locked_asset_token_nonce,
            &caller,
        )?;

        if unused_minted_assets > 0 {
            self.burn_tokens(&asset_token_id, &unused_minted_assets);
        }

        Ok(())
    }

    #[payable("*")]
    #[endpoint(removeLiquidityProxy)]
    fn remove_liquidity_proxy(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
        #[payment_nonce] token_nonce: Nonce,
        pair_address: Address,
        first_token_amount_min: Self::BigUint,
        second_token_amount_min: Self::BigUint,
    ) -> SCResult<()> {
        self.require_is_intermediated_pair(&pair_address)?;
        self.require_wrapped_lp_token_id_not_empty()?;
        require!(token_nonce != 0, "Can only be called with an SFT");
        require!(amount != 0, "Payment amount cannot be zero");

        let wrapped_lp_token_id = self.wrapped_lp_token_id().get();
        require!(token_id == wrapped_lp_token_id, "Wrong input token");

        let caller = self.blockchain().get_caller();
        let lp_token_id = self.ask_for_lp_token_id(&pair_address);
        let attributes = self.get_wrapped_lp_token_attributes(&token_id, token_nonce)?;
        require!(lp_token_id == attributes.lp_token_id, "Bad input address");

        let locked_asset_token_id = self.locked_asset_token_id().get();
        let asset_token_id = self.asset_token_id().get();

        self.reset_received_funds_on_current_tx();
        let tokens_for_position = self
            .actual_remove_liquidity(
                &pair_address,
                &lp_token_id,
                &amount,
                &first_token_amount_min,
                &second_token_amount_min,
            )
            .into_tuple();
        self.validate_received_funds_chunk(
            [
                (
                    &tokens_for_position.0.token_id,
                    0,
                    &tokens_for_position.0.amount,
                ),
                (
                    &tokens_for_position.1.token_id,
                    0,
                    &tokens_for_position.1.amount,
                ),
            ]
            .to_vec(),
        )?;

        let fungible_token_id: TokenIdentifier;
        let fungible_token_amount: Self::BigUint;
        let assets_received: Self::BigUint;
        let locked_assets_invested = self.rule_of_three(
            &amount,
            &attributes.lp_token_total_amount,
            &attributes.locked_assets_invested,
        );
        require!(
            locked_assets_invested > 0,
            "Not enough wrapped lp token provided"
        );
        if tokens_for_position.0.token_id == asset_token_id {
            assets_received = tokens_for_position.0.amount;
            fungible_token_id = tokens_for_position.1.token_id;
            fungible_token_amount = tokens_for_position.1.amount;
        } else if tokens_for_position.1.token_id == asset_token_id {
            assets_received = tokens_for_position.1.amount;
            fungible_token_id = tokens_for_position.0.token_id;
            fungible_token_amount = tokens_for_position.0.amount;
        } else {
            return sc_error!("Bad tokens received from pair SC");
        }

        //Send back the tokens removed from pair sc.
        self.send()
            .direct(&caller, &fungible_token_id, &fungible_token_amount, &[]);
        let locked_assets_to_send =
            core::cmp::min(assets_received.clone(), locked_assets_invested.clone());
        self.send().direct_nft(
            &caller,
            &locked_asset_token_id,
            attributes.locked_assets_nonce,
            &locked_assets_to_send,
            &[],
        );

        //Do cleanup
        if assets_received > locked_assets_invested {
            let difference = assets_received - locked_assets_invested;
            self.send()
                .direct(&caller, &asset_token_id, &difference, &[]);
        } else if assets_received < locked_assets_invested {
            let difference = locked_assets_invested - assets_received;
            self.nft_burn_tokens(
                &locked_asset_token_id,
                attributes.locked_assets_nonce,
                &difference,
            );
        }

        self.burn_tokens(&asset_token_id, &locked_assets_to_send);
        self.nft_burn_tokens(&wrapped_lp_token_id, token_nonce, &amount);
        Ok(())
    }

    fn actual_add_liquidity(
        &self,
        pair_address: &Address,
        first_token_amount_desired: &Self::BigUint,
        first_token_amount_min: &Self::BigUint,
        second_token_amount_desired: &Self::BigUint,
        second_token_amount_min: &Self::BigUint,
    ) -> AddLiquidityResultType<Self::BigUint> {
        self.pair_contract_proxy(pair_address.clone())
            .addLiquidity(
                first_token_amount_desired.clone(),
                second_token_amount_desired.clone(),
                first_token_amount_min.clone(),
                second_token_amount_min.clone(),
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context()
    }

    fn actual_remove_liquidity(
        &self,
        pair_address: &Address,
        lp_token_id: &TokenIdentifier,
        liquidity: &Self::BigUint,
        first_token_amount_min: &Self::BigUint,
        second_token_amount_min: &Self::BigUint,
    ) -> RemoveLiquidityResultType<Self::BigUint> {
        self.pair_contract_proxy(pair_address.clone())
            .removeLiquidity(
                lp_token_id.clone(),
                liquidity.clone(),
                first_token_amount_min.clone(),
                second_token_amount_min.clone(),
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context()
    }

    fn ask_for_lp_token_id(&self, pair_address: &Address) -> TokenIdentifier {
        self.pair_contract_proxy(pair_address.clone())
            .getLpTokenIdentifier()
            .execute_on_dest_context()
    }

    fn create_by_merging_and_send(
        &self,
        lp_token_id: &TokenIdentifier,
        lp_token_amount: &Self::BigUint,
        locked_tokens_consumed: &Self::BigUint,
        locked_tokens_nonce: Nonce,
        caller: &Address,
    ) -> SCResult<()> {
        self.merge_wrapped_lp_tokens_and_send(
            caller,
            Option::Some(WrappedLpToken {
                token_amount: GenericTokenAmountPair {
                    token_id: self.wrapped_lp_token_id().get(),
                    token_nonce: 0,
                    amount: lp_token_amount.clone(),
                },
                attributes: WrappedLpTokenAttributes {
                    lp_token_id: lp_token_id.clone(),
                    lp_token_total_amount: lp_token_amount.clone(),
                    locked_assets_invested: locked_tokens_consumed.clone(),
                    locked_assets_nonce: locked_tokens_nonce,
                },
            }),
            OptionalArg::None,
        )
    }

    fn forward_to_pair(
        &self,
        pair_address: &Address,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
        amount: &Self::BigUint,
    ) {
        let token_to_send: TokenIdentifier;
        if token_nonce == 0 {
            token_to_send = token_id.clone();
        } else {
            let asset_token_id = self.asset_token_id().get();
            self.mint_tokens(&asset_token_id, amount);
            token_to_send = asset_token_id;
        };
        self.pair_contract_proxy(pair_address.clone())
            .acceptEsdtPayment(token_to_send, amount.clone())
            .execute_on_dest_context();
    }

    fn increase_temporary_funds_amount(
        &self,
        caller: &Address,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
        increase_amount: &Self::BigUint,
    ) {
        let old_value = self
            .temporary_funds(caller)
            .get(&(token_id.clone(), token_nonce))
            .unwrap_or_else(Self::BigUint::zero);
        self.temporary_funds(caller).insert(
            (token_id.clone(), token_nonce),
            &old_value + increase_amount,
        );
    }

    fn decrease_temporary_funds_amount(
        &self,
        caller: &Address,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
        decrease_amount: &Self::BigUint,
    ) {
        let old_value = self
            .temporary_funds(caller)
            .get(&(token_id.clone(), token_nonce))
            .unwrap();

        if &old_value != decrease_amount {
            self.temporary_funds(caller).insert(
                (token_id.clone(), token_nonce),
                &old_value - decrease_amount,
            );
        } else {
            self.temporary_funds(caller)
                .remove(&(token_id.clone(), token_nonce));
        }
    }

    fn require_is_intermediated_pair(&self, address: &Address) -> SCResult<()> {
        require!(
            self.intermediated_pairs().contains(address),
            "Not an intermediated pair"
        );
        Ok(())
    }

    fn require_wrapped_lp_token_id_not_empty(&self) -> SCResult<()> {
        require!(!self.wrapped_lp_token_id().is_empty(), "Empty token id");
        Ok(())
    }

    #[view(getTemporaryFunds)]
    fn get_temporary_funds(
        &self,
        address: &Address,
    ) -> MultiResultVec<GenericTokenAmountPair<Self::BigUint>> {
        MultiResultVec::from_iter(
            self.temporary_funds(address)
                .iter()
                .map(|x| {
                    let (key, amount) = x;
                    let (token_id, token_nonce) = key;
                    GenericTokenAmountPair {
                        token_id,
                        token_nonce,
                        amount,
                    }
                })
                .collect::<Vec<GenericTokenAmountPair<Self::BigUint>>>(),
        )
    }

    #[storage_mapper("funds")]
    fn temporary_funds(
        &self,
        user: &Address,
    ) -> MapMapper<Self::Storage, (TokenIdentifier, Nonce), Self::BigUint>;
}
