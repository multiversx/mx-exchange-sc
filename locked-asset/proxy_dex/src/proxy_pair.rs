#![allow(clippy::too_many_arguments)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    pair_interactions::{CallAddLiqArgs, CallRemoveLiqArgs},
    wrapped_lp_attributes::{WrappedLpToken, WrappedLpTokenAttributes},
};
use common_structs::Epoch;
use fixed_supply_token::FixedSupplyToken;

#[multiversx_sc::module]
pub trait ProxyPairModule:
    crate::proxy_common::ProxyCommonModule
    + crate::other_sc_whitelist::OtherScWhitelistModule
    + crate::pair_interactions::PairInteractionsModule
    + crate::wrapped_lp_token_merge::WrappedLpTokenMerge
    + crate::energy_update::EnergyUpdateModule
    + crate::events::EventsModule
    + energy_query::EnergyQueryModule
    + token_merge_helper::TokenMergeHelperModule
    + token_send::TokenSendModule
    + utils::UtilsModule
    + legacy_token_decode_module::LegacyTokenDecodeModule
    + disable_add_liq::DisableAddLiqModule
{
    #[payable("*")]
    #[endpoint(addLiquidityProxy)]
    fn add_liquidity_proxy(
        &self,
        pair_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> MultiValueEncoded<EsdtTokenPayment> {
        self.require_is_intermediated_pair(&pair_address);
        self.require_wrapped_lp_token_id_not_empty();
        self.require_add_liq_enabled();

        let caller = self.blockchain().get_caller();
        let mut payments = self.get_non_empty_payments();
        let first_payment = self.pop_first_payment(&mut payments);
        let second_payment = self.pop_first_payment(&mut payments);

        let input_token_refs = self.require_exactly_one_locked(&first_payment, &second_payment);
        let asset_amount = input_token_refs.locked_token_ref.amount.clone();
        let asset_token_id = self.get_base_token_id();
        self.send()
            .esdt_local_mint(&asset_token_id, 0, &asset_amount);

        let first_unlocked_token_id =
            self.get_underlying_token(first_payment.token_identifier.clone());
        let second_unlocked_token_id =
            self.get_underlying_token(second_payment.token_identifier.clone());
        let add_liq_result = self.call_add_liquidity(CallAddLiqArgs {
            pair_address: pair_address.clone(),
            first_token_id: first_unlocked_token_id,
            first_token_amount_desired: first_payment.amount.clone(),
            first_token_amount_min,
            second_token_id: second_unlocked_token_id,
            second_token_amount_desired: second_payment.amount.clone(),
            second_token_amount_min,
        });

        let mut locked_token_used = input_token_refs.locked_token_ref.clone();
        locked_token_used.amount = if input_token_refs.locked_token_ref.token_identifier
            == first_payment.token_identifier
        {
            first_payment.amount.clone() - &add_liq_result.first_token_leftover.amount
        } else {
            second_payment.amount.clone() - &add_liq_result.second_token_leftover.amount
        };

        let new_token_attributes = WrappedLpTokenAttributes {
            locked_tokens: locked_token_used,
            lp_token_id: add_liq_result.lp_tokens_received.token_identifier.clone(),
            lp_token_amount: add_liq_result.lp_tokens_received.amount.clone(),
        };

        let wrapped_lp_mapper = self.wrapped_lp_token();
        let token_merge_requested = !payments.is_empty();
        let new_wrapped_token = if token_merge_requested {
            let wrapped_lp_tokens =
                WrappedLpToken::new_from_payments(&payments, &wrapped_lp_mapper);

            self.send().esdt_local_burn_multi(&payments);

            self.merge_wrapped_lp_tokens_with_virtual_pos(
                &caller,
                wrapped_lp_tokens,
                new_token_attributes,
            )
        } else {
            let new_token_amount = new_token_attributes.get_total_supply();
            let output_wrapped_lp_token =
                wrapped_lp_mapper.nft_create(new_token_amount, &new_token_attributes);

            WrappedLpToken {
                payment: output_wrapped_lp_token,
                attributes: new_token_attributes,
            }
        };

        let received_token_refs = self.require_exactly_one_base_asset(
            &add_liq_result.first_token_leftover,
            &add_liq_result.second_token_leftover,
        );
        let other_token_leftover = received_token_refs.other_token_ref.clone();
        let mut locked_token_leftover = input_token_refs.locked_token_ref.clone();
        locked_token_leftover.amount = received_token_refs.base_asset_token_ref.amount.clone();

        if locked_token_leftover.amount > 0 {
            self.send()
                .esdt_local_burn(&asset_token_id, 0, &locked_token_leftover.amount);
        }

        let mut output_payments = ManagedVec::new();
        output_payments.push(new_wrapped_token.payment.clone());
        output_payments.push(locked_token_leftover);
        output_payments.push(other_token_leftover);

        self.send_multiple_tokens_if_not_zero(&caller, &output_payments);

        self.emit_add_liquidity_proxy_event(
            &caller,
            &pair_address,
            first_payment,
            second_payment,
            new_wrapped_token.payment.clone(),
            new_wrapped_token.attributes,
            token_merge_requested,
        );

        output_payments.into()
    }

    #[payable("*")]
    #[endpoint(removeLiquidityProxy)]
    fn remove_liquidity_proxy(
        &self,
        pair_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> MultiValueEncoded<EsdtTokenPayment> {
        self.require_is_intermediated_pair(&pair_address);
        self.require_wrapped_lp_token_id_not_empty();

        let payment = self.call_value().single_esdt();

        let output_payments = self.remove_liquidity_proxy_common(
            payment,
            pair_address,
            first_token_amount_min,
            second_token_amount_min,
        );
        let caller = self.blockchain().get_caller();
        self.send_multiple_tokens_if_not_zero(&caller, &output_payments);

        output_payments.into()
    }

    fn remove_liquidity_proxy_common(
        &self,
        input_payment: EsdtTokenPayment,
        pair_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> ManagedVec<EsdtTokenPayment> {
        let wrapped_lp_mapper = self.wrapped_lp_token();
        wrapped_lp_mapper.require_same_token(&input_payment.token_identifier);

        let caller = self.blockchain().get_caller();
        let attributes: WrappedLpTokenAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(&input_payment, &wrapped_lp_mapper);

        let remove_liq_result = self.call_remove_liquidity(CallRemoveLiqArgs {
            pair_address: pair_address.clone(),
            lp_token_id: attributes.lp_token_id.clone(),
            lp_token_amount: attributes.lp_token_amount.clone(),
            first_token_amount_min,
            second_token_amount_min,
        });
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
            self.burn_locked_tokens_and_update_energy(
                &attributes.locked_tokens.token_identifier,
                attributes.locked_tokens.token_nonce,
                &extra_locked_tokens,
                &caller,
            );

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

        wrapped_lp_mapper.nft_burn(input_payment.token_nonce, &input_payment.amount);

        self.emit_remove_liquidity_proxy_event(
            &caller,
            &pair_address,
            input_payment,
            attributes,
            remove_liq_result.first_token_received,
            remove_liq_result.second_token_received,
        );

        output_payments
    }

    #[payable("*")]
    #[endpoint(increaseProxyPairTokenEnergy)]
    fn increase_proxy_pair_token_energy_endpoint(&self, lock_epochs: Epoch) -> EsdtTokenPayment {
        self.require_wrapped_lp_token_id_not_empty();

        let payment = self.call_value().single_esdt();
        let wrapped_lp_mapper = self.wrapped_lp_token();
        wrapped_lp_mapper.require_same_token(&payment.token_identifier);

        let caller = self.blockchain().get_caller();
        let old_attributes: WrappedLpTokenAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(&payment, &wrapped_lp_mapper);

        let new_locked_tokens =
            self.increase_proxy_pair_token_energy(caller.clone(), lock_epochs, &old_attributes);
        let new_token_attributes = WrappedLpTokenAttributes {
            locked_tokens: new_locked_tokens,
            lp_token_id: old_attributes.lp_token_id,
            lp_token_amount: old_attributes.lp_token_amount,
        };

        self.send().esdt_local_burn(
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );

        let new_token_amount = new_token_attributes.get_total_supply();

        wrapped_lp_mapper.nft_create_and_send(&caller, new_token_amount, &new_token_attributes)
    }

    fn increase_proxy_pair_token_energy(
        &self,
        user: ManagedAddress,
        lock_epochs: Epoch,
        old_attributes: &WrappedLpTokenAttributes<Self::Api>,
    ) -> EsdtTokenPayment {
        let new_locked_token_id = self.get_locked_token_id();
        require!(
            old_attributes.locked_tokens.token_identifier == new_locked_token_id,
            "Invalid payment"
        );

        let energy_factory_addr = self.energy_factory_address().get();
        self.call_increase_energy(
            user,
            old_attributes.locked_tokens.clone(),
            lock_epochs,
            energy_factory_addr,
        )
    }

    fn require_wrapped_lp_token_id_not_empty(&self) {
        require!(!self.wrapped_lp_token().is_empty(), "Empty token id");
    }
}
