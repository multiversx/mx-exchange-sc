#![allow(clippy::too_many_arguments)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::vec_init_then_push)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{Nonce, WrappedLpTokenAttributes};
use itertools::Itertools;
use pair::config::ProxyTrait as _;
use pair::ProxyTrait as _;

use super::events;
use super::proxy_common;
use super::wrapped_lp_token_merge;

type AddLiquidityResultType<BigUint> =
    MultiValue3<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

type RemoveLiquidityResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[derive(ManagedVecItem, Clone)]
pub struct WrappedLpToken<M: ManagedTypeApi> {
    pub token: EsdtTokenPayment<M>,
    pub attributes: WrappedLpTokenAttributes<M>,
}

#[elrond_wasm::module]
pub trait ProxyPairModule:
    proxy_common::ProxyCommonModule
    + wrapped_lp_token_merge::WrappedLpTokenMerge
    + token_merge_helper::TokenMergeHelperModule
    + token_send::TokenSendModule
    + events::EventsModule
{
    #[only_owner]
    #[endpoint(addPairToIntermediate)]
    fn add_pair_to_intermediate(&self, pair_address: ManagedAddress) {
        self.intermediated_pairs().insert(pair_address);
    }

    #[only_owner]
    #[endpoint(removeIntermediatedPair)]
    fn remove_intermediated_pair(&self, pair_address: ManagedAddress) {
        self.require_is_intermediated_pair(&pair_address);
        self.intermediated_pairs().remove(&pair_address);
    }

    #[payable("*")]
    #[endpoint(addLiquidityProxy)]
    fn add_liquidity_proxy(
        &self,
        pair_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> AddLiquidityResultType<Self::Api> {
        self.require_is_intermediated_pair(&pair_address);
        self.require_wrapped_lp_token_id_not_empty();

        let payments_vec = self.call_value().all_esdt_transfers();
        let mut payments_iter = payments_vec.iter();
        let (payment_0, payment_1) = payments_iter
            .next_tuple()
            .unwrap_or_else(|| sc_panic!("bad payment len"));

        let first_token_id = payment_0.token_identifier.clone();
        let first_token_nonce = payment_0.token_nonce;
        let first_token_amount_desired = payment_0.amount;
        require!(first_token_nonce == 0, "bad first token nonce");
        require!(
            first_token_amount_desired > 0u32,
            "first payment amount zero"
        );
        require!(
            first_token_amount_desired >= first_token_amount_min,
            "bad first token min"
        );

        let second_token_id = payment_1.token_identifier.clone();
        let second_token_nonce = payment_1.token_nonce;
        let second_token_amount_desired = payment_1.amount;
        require!(
            second_token_id == self.locked_asset_token_id().get(),
            "second token needs to be locked asset token"
        );
        require!(second_token_nonce != 0, "bad second token nonce");
        require!(
            second_token_amount_desired > 0u32,
            "second payment amount zero"
        );
        require!(
            second_token_amount_desired >= second_token_amount_min,
            "bad second token min"
        );

        let asset_token_id = self.asset_token_id().get();
        self.send()
            .esdt_local_mint(&asset_token_id, 0, &second_token_amount_desired);

        let result = self.actual_add_liquidity(
            &pair_address,
            &first_token_id,
            &first_token_amount_desired,
            &first_token_amount_min,
            &asset_token_id,
            &second_token_amount_desired,
            &second_token_amount_min,
        );

        let result_tuple = result.0;
        let lp_received = result_tuple.0;
        let first_token_used = result_tuple.1;
        let second_token_used = result_tuple.2;
        require!(
            lp_received.amount > 0u32,
            "LP token amount should be greater than 0"
        );
        require!(
            first_token_used.amount <= first_token_amount_desired,
            "Used more first tokens than provided"
        );
        require!(
            second_token_used.amount <= second_token_amount_desired,
            "Used more second tokens than provided"
        );

        let caller = self.blockchain().get_caller();
        let (new_wrapped_lp_token, created_with_merge) = self.create_by_merging_and_send(
            &lp_received.token_identifier,
            &lp_received.amount,
            &second_token_used.amount,
            second_token_nonce,
            &caller,
            payments_iter,
        );

        let mut surplus_payments = ManagedVec::new();
        surplus_payments.push(EsdtTokenPayment::new(
            first_token_id.clone(),
            0,
            &first_token_amount_desired - &first_token_used.amount,
        ));
        surplus_payments.push(EsdtTokenPayment::new(
            second_token_id.clone(),
            second_token_nonce,
            &second_token_amount_desired - &second_token_used.amount,
        ));
        self.send_multiple_tokens_if_not_zero(&caller, &surplus_payments);

        if second_token_amount_desired > second_token_used.amount {
            let unused_minted_assets = &second_token_amount_desired - &second_token_used.amount;
            self.send()
                .esdt_local_burn(&asset_token_id, 0, &unused_minted_assets);
        }

        self.emit_add_liquidity_proxy_event(
            &caller,
            &pair_address,
            &first_token_id,
            first_token_nonce,
            &first_token_used.amount,
            &second_token_id,
            first_token_nonce,
            &second_token_used.amount,
            &new_wrapped_lp_token.token.token_identifier,
            new_wrapped_lp_token.token.token_nonce,
            &new_wrapped_lp_token.token.amount,
            &new_wrapped_lp_token.attributes,
            created_with_merge,
        );

        (
            new_wrapped_lp_token.token,
            surplus_payments.get(0),
            surplus_payments.get(1),
        )
            .into()
    }

    #[payable("*")]
    #[endpoint(removeLiquidityProxy)]
    fn remove_liquidity_proxy(
        &self,
        pair_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> RemoveLiquidityResultType<Self::Api> {
        let (token_id, token_nonce, amount) = self.call_value().single_esdt().into_tuple();

        self.require_is_intermediated_pair(&pair_address);
        self.require_wrapped_lp_token_id_not_empty();
        require!(token_nonce != 0, "Can only be called with an SFT");
        require!(amount != 0, "Payment amount cannot be zero");

        let wrapped_lp_token_id = self.wrapped_lp_token().get_token_id();
        require!(token_id == wrapped_lp_token_id, "Wrong input token");

        let caller = self.blockchain().get_caller();
        let lp_token_id = self.ask_for_lp_token_id(&pair_address);
        let attributes = self.get_wrapped_lp_token_attributes(&token_id, token_nonce);
        require!(lp_token_id == attributes.lp_token_id, "Bad input address");

        let locked_asset_token_id = self.locked_asset_token_id().get();
        let asset_token_id = self.asset_token_id().get();

        let tokens_for_position = self
            .actual_remove_liquidity(
                &pair_address,
                &lp_token_id,
                &amount,
                &first_token_amount_min,
                &second_token_amount_min,
            )
            .into_tuple();

        let fungible_token_id: TokenIdentifier;
        let fungible_token_amount: BigUint;
        let assets_received: BigUint;
        let locked_assets_invested = self.rule_of_three_non_zero_result(
            &amount,
            &attributes.lp_token_total_amount,
            &attributes.locked_assets_invested,
        );

        if tokens_for_position.1.token_identifier == asset_token_id {
            assets_received = tokens_for_position.1.amount.clone();
            fungible_token_id = tokens_for_position.0.token_identifier.clone();
            fungible_token_amount = tokens_for_position.0.amount.clone();
        } else {
            sc_panic!("Bad tokens received from pair SC");
        }

        //Send back the tokens removed from pair sc.
        self.send()
            .direct_esdt(&caller, &fungible_token_id, 0, &fungible_token_amount);
        let locked_assets_to_send =
            core::cmp::min(assets_received.clone(), locked_assets_invested.clone());
        self.send().direct_esdt(
            &caller,
            &locked_asset_token_id,
            attributes.locked_assets_nonce,
            &locked_assets_to_send,
        );

        //Do cleanup
        if assets_received > locked_assets_invested {
            let difference = assets_received - locked_assets_invested;
            self.send()
                .direct_esdt(&caller, &asset_token_id, 0, &difference);
        } else if assets_received < locked_assets_invested {
            let difference = locked_assets_invested - assets_received;
            self.send().esdt_local_burn(
                &locked_asset_token_id,
                attributes.locked_assets_nonce,
                &difference,
            );
        }

        self.send()
            .esdt_local_burn(&asset_token_id, 0, &locked_assets_to_send);
        self.send()
            .esdt_local_burn(&wrapped_lp_token_id, token_nonce, &amount);

        self.emit_remove_liquidity_proxy_event(
            &caller,
            &pair_address,
            &token_id,
            token_nonce,
            &amount,
            &attributes,
            &tokens_for_position.0.token_identifier,
            0,
            &tokens_for_position.0.amount,
            &tokens_for_position.1.token_identifier,
            0,
            &tokens_for_position.1.amount,
        );

        tokens_for_position.into()
    }

    fn actual_add_liquidity(
        &self,
        pair_address: &ManagedAddress,
        first_token_id: &TokenIdentifier,
        first_token_amount_desired: &BigUint,
        first_token_amount_min: &BigUint,
        second_token_id: &TokenIdentifier,
        second_token_amount_desired: &BigUint,
        second_token_amount_min: &BigUint,
    ) -> AddLiquidityResultType<Self::Api> {
        let mut all_token_payments = ManagedVec::new();

        let first_payment = EsdtTokenPayment::new(
            first_token_id.clone(),
            0,
            first_token_amount_desired.clone(),
        );
        all_token_payments.push(first_payment);

        let second_payment = EsdtTokenPayment::new(
            second_token_id.clone(),
            0,
            second_token_amount_desired.clone(),
        );
        all_token_payments.push(second_payment);

        self.pair_contract_proxy(pair_address.clone())
            .add_liquidity(
                first_token_amount_min.clone(),
                second_token_amount_min.clone(),
            )
            .with_multi_token_transfer(all_token_payments)
            .execute_on_dest_context()
    }

    fn actual_remove_liquidity(
        &self,
        pair_address: &ManagedAddress,
        lp_token_id: &TokenIdentifier,
        liquidity: &BigUint,
        first_token_amount_min: &BigUint,
        second_token_amount_min: &BigUint,
    ) -> RemoveLiquidityResultType<Self::Api> {
        self.pair_contract_proxy(pair_address.clone())
            .remove_liquidity(
                first_token_amount_min.clone(),
                second_token_amount_min.clone(),
            )
            .add_esdt_token_transfer(lp_token_id.clone(), 0, liquidity.clone())
            .execute_on_dest_context()
    }

    fn ask_for_lp_token_id(&self, pair_address: &ManagedAddress) -> TokenIdentifier {
        self.pair_contract_proxy(pair_address.clone())
            .get_lp_token_identifier()
            .execute_on_dest_context()
    }

    fn create_by_merging_and_send(
        &self,
        lp_token_id: &TokenIdentifier,
        lp_token_amount: &BigUint,
        locked_tokens_consumed: &BigUint,
        locked_tokens_nonce: Nonce,
        caller: &ManagedAddress,
        additional_payments: ManagedVecRefIterator<Self::Api, EsdtTokenPayment<Self::Api>>,
    ) -> (WrappedLpToken<Self::Api>, bool) {
        self.merge_wrapped_lp_tokens_and_send(
            caller,
            additional_payments,
            Option::Some(WrappedLpToken {
                token: EsdtTokenPayment::new(
                    self.wrapped_lp_token().get_token_id(),
                    0,
                    lp_token_amount.clone(),
                ),
                attributes: WrappedLpTokenAttributes {
                    lp_token_id: lp_token_id.clone(),
                    lp_token_total_amount: lp_token_amount.clone(),
                    locked_assets_invested: locked_tokens_consumed.clone(),
                    locked_assets_nonce: locked_tokens_nonce,
                },
            }),
        )
    }

    fn require_is_intermediated_pair(&self, address: &ManagedAddress) {
        require!(
            self.intermediated_pairs().contains(address),
            "Not an intermediated pair"
        );
    }

    fn require_wrapped_lp_token_id_not_empty(&self) {
        require!(!self.wrapped_lp_token().is_empty(), "Empty token id");
    }
}
