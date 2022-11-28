#![allow(clippy::too_many_arguments)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::vec_init_then_push)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::WrappedLpTokenAttributes;
use common_structs::{RawResultWrapper, RawResultsType};

use super::events;
use super::proxy_common;

type RemoveLiquidityResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

mod pair_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait PairProxy {
        #[payable("*")]
        #[endpoint(removeLiquidity)]
        fn remove_liquidity(
            &self,
            first_token_amount_min: BigUint,
            second_token_amount_min: BigUint,
        ) -> super::RemoveLiquidityResultType<Self::Api>;

        #[view(getLpTokenIdentifier)]
        fn get_lp_token_identifier(&self) -> TokenIdentifier;
    }
}

#[derive(ManagedVecItem, Clone)]
pub struct WrappedLpToken<M: ManagedTypeApi> {
    pub token_amount: EsdtTokenPayment<M>,
    pub attributes: WrappedLpTokenAttributes<M>,
}

#[elrond_wasm::module]
pub trait ProxyPairModule:
    proxy_common::ProxyCommonModule + token_merge::TokenMergeModule + events::EventsModule
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
    #[endpoint(removeLiquidityProxy)]
    fn remove_liquidity_proxy(
        &self,
        pair_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) {
        self.require_is_intermediated_pair(&pair_address);
        self.require_wrapped_lp_token_id_not_empty();

        let (token_id, token_nonce, amount) = self.call_value().single_esdt().into_tuple();
        require!(token_nonce != 0, "Can only be called with an SFT");
        require!(amount != 0, "Payment amount cannot be zero");

        let wrapped_lp_token_id = self.wrapped_lp_token_id().get();
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

        // Send back the tokens removed from pair sc.
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

        // Do cleanup
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
    }

    fn actual_remove_liquidity(
        &self,
        pair_address: &ManagedAddress,
        lp_token_id: &TokenIdentifier,
        liquidity: &BigUint,
        first_token_amount_min: &BigUint,
        second_token_amount_min: &BigUint,
    ) -> RemoveLiquidityResultType<Self::Api> {
        let raw_results: RawResultsType<Self::Api> = self
            .pair_contract_proxy(pair_address.clone())
            .remove_liquidity(
                first_token_amount_min.clone(),
                second_token_amount_min.clone(),
            )
            .add_esdt_token_transfer(lp_token_id.clone(), 0, liquidity.clone())
            .execute_on_dest_context();

        let mut results_wrapper = RawResultWrapper::new(raw_results);
        results_wrapper.trim_results_front(2);

        let first_token = results_wrapper.decode_next_result();
        let second_token = results_wrapper.decode_next_result();

        (first_token, second_token).into()
    }

    fn ask_for_lp_token_id(&self, pair_address: &ManagedAddress) -> TokenIdentifier {
        self.pair_contract_proxy(pair_address.clone())
            .get_lp_token_identifier()
            .execute_on_dest_context()
    }

    fn require_is_intermediated_pair(&self, address: &ManagedAddress) {
        require!(
            self.intermediated_pairs().contains(address),
            "Not an intermediated pair"
        );
    }

    fn require_wrapped_lp_token_id_not_empty(&self) {
        require!(!self.wrapped_lp_token_id().is_empty(), "Empty token id");
    }

    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair_proxy::Proxy<Self::Api>;
}
