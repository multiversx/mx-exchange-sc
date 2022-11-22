use common_structs::{RawResultWrapper, RawResultsType};
use proxy_dex::{
    pair_interactions::RemoveLiqudityResultWrapper, wrapped_lp_attributes::WrappedLpTokenAttributes,
};

elrond_wasm::imports!();

const REMOVE_LIQ_RESULTS_LEN: usize = 2;

mod old_pair_proxy {
    elrond_wasm::imports!();

    use common_structs::RawResultsType;

    #[elrond_wasm::proxy]
    pub trait OldPairProxy {
        #[payable("*")]
        #[endpoint(removeLiquidity)]
        fn remove_liquidity(
            &self,
            first_token_amount_min: BigUint,
            second_token_amount_min: BigUint,
        ) -> RawResultsType<Self::Api>;
    }
}

#[elrond_wasm::module]
pub trait ExitPairModule:
    proxy_dex::proxy_common::ProxyCommonModule
    + proxy_dex::events::EventsModule
    + proxy_dex::sc_whitelist::ScWhitelistModule
    + token_send::TokenSendModule
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(removeLiquidityProxy)]
    fn remove_liquidity_proxy(
        &self,
        pair_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> MultiValueEncoded<EsdtTokenPayment> {
        self.require_is_intermediated_pair(&pair_address);

        let payment = self.call_value().single_esdt();
        let wrapped_lp_mapper = self.wrapped_lp_token();
        wrapped_lp_mapper.require_same_token(&payment.token_identifier);

        let caller = self.blockchain().get_caller();
        let attributes: WrappedLpTokenAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(&payment, &wrapped_lp_mapper);

        let remove_liq_result = self.call_remove_liquidity(
            pair_address.clone(),
            attributes.lp_token_id.clone(),
            attributes.lp_token_amount.clone(),
            first_token_amount_min,
            second_token_amount_min,
        );
        let received_token_refs = self.require_exactly_one_base_asset(
            &remove_liq_result.first_token_received,
            &remove_liq_result.second_token_received,
        );

        let mut output_payments = ManagedVec::new();

        let base_asset_amount_received = &received_token_refs.base_asset_token_ref.amount.clone();
        let locked_token_amount_available = &attributes.locked_tokens.amount;
        if base_asset_amount_received > locked_token_amount_available {
            let asset_token_id = received_token_refs
                .base_asset_token_ref
                .token_identifier
                .clone();
            let unlocked_amount = base_asset_amount_received - locked_token_amount_available;
            let unlocked_tokens = EsdtTokenPayment::new(asset_token_id.clone(), 0, unlocked_amount);

            // burn base asset, as we only need to send the locked tokens
            self.send()
                .esdt_local_burn(&asset_token_id, 0, &attributes.locked_tokens.amount);

            output_payments.push(unlocked_tokens);
            output_payments.push(attributes.locked_tokens.clone());
        } else {
            let extra_locked_tokens = locked_token_amount_available - base_asset_amount_received;
            if extra_locked_tokens > 0 {
                self.send().esdt_local_burn(
                    &attributes.locked_tokens.token_identifier,
                    attributes.locked_tokens.token_nonce,
                    &extra_locked_tokens,
                );
            }

            let mut locked_tokens_out = attributes.locked_tokens.clone();
            locked_tokens_out.amount = base_asset_amount_received.clone();

            // burn base asset, as we only need to send the locked tokens
            let asset_token_id = received_token_refs
                .base_asset_token_ref
                .token_identifier
                .clone();
            self.send()
                .esdt_local_burn(&asset_token_id, 0, &locked_tokens_out.amount);

            output_payments.push(locked_tokens_out);
        }

        let other_tokens = received_token_refs.other_token_ref.clone();
        output_payments.push(other_tokens);

        wrapped_lp_mapper.nft_burn(payment.token_nonce, &payment.amount);

        self.send_multiple_tokens_if_not_zero(&caller, &output_payments);

        self.emit_remove_liquidity_proxy_event(
            &caller,
            &pair_address,
            payment,
            attributes,
            remove_liq_result.first_token_received,
            remove_liq_result.second_token_received,
        );

        output_payments.into()
    }

    fn call_remove_liquidity(
        &self,
        pair_address: ManagedAddress,
        lp_token_id: TokenIdentifier,
        lp_token_amount: BigUint,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> RemoveLiqudityResultWrapper<Self::Api> {
        let raw_results: RawResultsType<Self::Api> = self
            .old_pair_proxy_obj(pair_address)
            .remove_liquidity(first_token_amount_min, second_token_amount_min)
            .add_esdt_token_transfer(lp_token_id, 0, lp_token_amount)
            .execute_on_dest_context();

        let mut results_wrapper = RawResultWrapper::new(raw_results);
        results_wrapper.trim_results_front(REMOVE_LIQ_RESULTS_LEN);

        let first_token_received = results_wrapper.decode_next_result();
        let second_token_received = results_wrapper.decode_next_result();

        RemoveLiqudityResultWrapper {
            first_token_received,
            second_token_received,
        }
    }

    #[proxy]
    fn old_pair_proxy_obj(&self, sc_address: ManagedAddress) -> old_pair_proxy::Proxy<Self::Api>;
}
