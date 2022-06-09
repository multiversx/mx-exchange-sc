#![allow(clippy::too_many_arguments)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{Nonce, WrappedFarmTokenAttributes};

use super::events;
use super::proxy_common;
use super::proxy_pair;
use super::wrapped_farm_token_merge;
use super::wrapped_lp_token_merge;
use farm::ProxyTrait as _;

type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[derive(ManagedVecItem, Clone)]
pub struct WrappedFarmToken<M: ManagedTypeApi> {
    pub token: EsdtTokenPayment<M>,
    pub attributes: WrappedFarmTokenAttributes<M>,
}

#[elrond_wasm::module]
pub trait ProxyFarmModule:
    proxy_common::ProxyCommonModule
    + proxy_pair::ProxyPairModule
    + token_merge::TokenMergeModule
    + token_send::TokenSendModule
    + wrapped_farm_token_merge::WrappedFarmTokenMerge
    + wrapped_lp_token_merge::WrappedLpTokenMerge
    + events::EventsModule
{
    #[only_owner]
    #[endpoint(addFarmToIntermediate)]
    fn add_farm_to_intermediate(&self, farm_address: ManagedAddress) {
        self.intermediated_farms().insert(farm_address);
    }

    #[only_owner]
    #[endpoint(removeIntermediatedFarm)]
    fn remove_intermediated_farm(&self, farm_address: ManagedAddress) {
        self.require_is_intermediated_farm(&farm_address);
        self.intermediated_farms().remove(&farm_address);
    }

    #[payable("*")]
    #[endpoint(enterFarmProxy)]
    fn enter_farm_proxy_endpoint(
        &self,
        farm_address: ManagedAddress,
    ) -> EnterFarmResultType<Self::Api> {
        self.require_is_intermediated_farm(&farm_address);
        self.require_wrapped_farm_token_id_not_empty();
        self.require_wrapped_lp_token_id_not_empty();

        let payments_vec = self.call_value().all_esdt_transfers();
        let mut payments_iter = payments_vec.iter();
        let payment_0 = payments_iter
            .next()
            .unwrap_or_else(|| sc_panic!("bad payment len"));

        let token_id = payment_0.token_identifier.clone();
        let token_nonce = payment_0.token_nonce;
        let amount = payment_0.amount;
        require!(amount != 0u32, "Payment amount cannot be zero");

        let farming_token_id: TokenIdentifier;
        if token_id == self.wrapped_lp_token().get_token_id() {
            let wrapped_lp_token_attrs =
                self.get_wrapped_lp_token_attributes(&token_id, token_nonce);
            farming_token_id = wrapped_lp_token_attrs.lp_token_id;
        } else if token_id == self.locked_asset_token_id().get() {
            let asset_token_id = self.asset_token_id().get();
            farming_token_id = asset_token_id;
        } else {
            sc_panic!("Unknown input Token");
        }

        let farm_result = self.actual_enter_farm(&farm_address, &farming_token_id, &amount);
        let farm_token_id = farm_result.token_identifier;
        let farm_token_nonce = farm_result.token_nonce;
        let farm_token_total_amount = farm_result.amount;
        require!(
            farm_token_total_amount > 0u32,
            "Farm token amount received should be greater than 0"
        );

        let attributes = WrappedFarmTokenAttributes {
            farm_token_id,
            farm_token_nonce,
            farm_token_amount: farm_token_total_amount.clone(),
            farming_token_id: token_id.clone(),
            farming_token_nonce: token_nonce,
            farming_token_amount: amount.clone(),
        };
        let caller = self.blockchain().get_caller();
        let (new_wrapped_farm_token, created_with_merge) = self
            .create_wrapped_farm_tokens_by_merging_and_send(
                &attributes,
                &farm_token_total_amount,
                &farm_address,
                &caller,
                payments_iter,
            );

        self.emit_enter_farm_proxy_event(
            &caller,
            &farm_address,
            &token_id,
            token_nonce,
            &amount,
            &new_wrapped_farm_token.token.token_identifier,
            new_wrapped_farm_token.token.token_nonce,
            &new_wrapped_farm_token.token.amount,
            &new_wrapped_farm_token.attributes,
            created_with_merge,
        );

        new_wrapped_farm_token.token
    }

    #[payable("*")]
    #[endpoint(exitFarmProxy)]
    fn exit_farm_proxy(&self, farm_address: ManagedAddress) -> ExitFarmResultType<Self::Api> {
        let (token_id, token_nonce, amount) = self.call_value().single_esdt().into_tuple();
        self.require_is_intermediated_farm(&farm_address);
        self.require_wrapped_farm_token_id_not_empty();
        self.require_wrapped_lp_token_id_not_empty();

        require!(amount != 0, "Payment amount cannot be zero");
        require!(
            token_id == self.wrapped_farm_token().get_token_id(),
            "Should only be used with wrapped farm tokens"
        );

        let wrapped_farm_token_attrs =
            self.get_wrapped_farm_token_attributes(&token_id, token_nonce);
        let farm_token_id = wrapped_farm_token_attrs.farm_token_id.clone();
        let farm_token_nonce = wrapped_farm_token_attrs.farm_token_nonce;

        let farm_result = self
            .actual_exit_farm(&farm_address, &farm_token_id, farm_token_nonce, &amount)
            .into_tuple();
        let farming_token_returned = farm_result.0;
        let reward_token_returned = farm_result.1;

        let caller = self.blockchain().get_caller();
        self.send().direct_esdt(
            &caller,
            &wrapped_farm_token_attrs.farming_token_id,
            wrapped_farm_token_attrs.farming_token_nonce,
            &farming_token_returned.amount,
            &[],
        );

        self.send_tokens_non_zero(
            &caller,
            &reward_token_returned.token_identifier,
            reward_token_returned.token_nonce,
            &reward_token_returned.amount,
        );
        self.send().esdt_local_burn(&token_id, token_nonce, &amount);

        if farming_token_returned.token_identifier == self.asset_token_id().get() {
            self.send().esdt_local_burn(
                &farming_token_returned.token_identifier,
                0,
                &farming_token_returned.amount,
            );
        }

        self.emit_exit_farm_proxy_event(
            &caller,
            &farm_address,
            &token_id,
            token_nonce,
            &amount,
            &wrapped_farm_token_attrs,
            &wrapped_farm_token_attrs.farming_token_id,
            wrapped_farm_token_attrs.farming_token_nonce,
            &farming_token_returned.amount,
            &reward_token_returned.token_identifier,
            reward_token_returned.token_nonce,
            &reward_token_returned.amount,
        );

        (
            EsdtTokenPayment::new(
                wrapped_farm_token_attrs.farming_token_id,
                wrapped_farm_token_attrs.farming_token_nonce,
                farming_token_returned.amount,
            ),
            reward_token_returned,
        )
            .into()
    }

    #[payable("*")]
    #[endpoint(claimRewardsProxy)]
    fn claim_rewards_proxy(
        &self,
        farm_address: ManagedAddress,
    ) -> ClaimRewardsResultType<Self::Api> {
        self.require_is_intermediated_farm(&farm_address);
        self.require_wrapped_farm_token_id_not_empty();
        self.require_wrapped_lp_token_id_not_empty();

        let payments_vec = self.call_value().all_esdt_transfers();
        let mut payments_iter = payments_vec.iter();
        let payment_0 = payments_iter
            .next()
            .unwrap_or_else(|| sc_panic!("bad payment len"));

        let token_id = payment_0.token_identifier.clone();
        let token_nonce = payment_0.token_nonce;
        let amount = payment_0.amount;
        require!(amount != 0u32, "Payment amount cannot be zero");

        require!(
            token_id == self.wrapped_farm_token().get_token_id(),
            "Should only be used with wrapped farm tokens"
        );

        // Read info about wrapped farm token and then burn it.
        let wrapped_farm_token_attrs =
            self.get_wrapped_farm_token_attributes(&token_id, token_nonce);
        let farm_token_id = wrapped_farm_token_attrs.farm_token_id.clone();
        let farm_token_nonce = wrapped_farm_token_attrs.farm_token_nonce;

        let result = self
            .actual_claim_rewards(&farm_address, &farm_token_id, farm_token_nonce, &amount)
            .into_tuple();
        let new_farm_token = result.0;
        let reward_token_returned = result.1;
        let new_farm_token_id = new_farm_token.token_identifier;
        let new_farm_token_nonce = new_farm_token.token_nonce;
        let new_farm_token_total_amount = new_farm_token.amount;
        require!(
            new_farm_token_total_amount > 0u32,
            "Farm token amount received should be greater than 0"
        );

        // Send the reward to the caller.
        let caller = self.blockchain().get_caller();
        self.send_tokens_non_zero(
            &caller,
            &reward_token_returned.token_identifier,
            reward_token_returned.token_nonce,
            &reward_token_returned.amount,
        );

        // Create new Wrapped tokens and send them.
        let new_wrapped_farm_token_attributes = WrappedFarmTokenAttributes {
            farm_token_id: new_farm_token_id,
            farm_token_nonce: new_farm_token_nonce,
            farm_token_amount: new_farm_token_total_amount.clone(),
            farming_token_id: wrapped_farm_token_attrs.farming_token_id.clone(),
            farming_token_nonce: wrapped_farm_token_attrs.farming_token_nonce,
            farming_token_amount: self.rule_of_three_non_zero_result(
                &amount,
                &wrapped_farm_token_attrs.farm_token_amount,
                &wrapped_farm_token_attrs.farming_token_amount,
            ),
        };
        let (new_wrapped_farm, created_with_merge) = self
            .create_wrapped_farm_tokens_by_merging_and_send(
                &new_wrapped_farm_token_attributes,
                &new_farm_token_total_amount,
                &farm_address,
                &caller,
                payments_iter,
            );
        self.send().esdt_local_burn(&token_id, token_nonce, &amount);

        self.emit_claim_rewards_farm_proxy_event(
            &caller,
            &farm_address,
            &token_id,
            token_nonce,
            &amount,
            &new_wrapped_farm.token.token_identifier,
            new_wrapped_farm.token.token_nonce,
            &new_wrapped_farm.token.amount,
            &reward_token_returned.token_identifier,
            reward_token_returned.token_nonce,
            &reward_token_returned.amount,
            &wrapped_farm_token_attrs,
            &new_wrapped_farm.attributes,
            created_with_merge,
        );

        (new_wrapped_farm.token, reward_token_returned).into()
    }

    #[payable("*")]
    #[endpoint(compoundRewardsProxy)]
    fn compound_rewards_proxy(
        &self,
        farm_address: ManagedAddress,
    ) -> CompoundRewardsResultType<Self::Api> {
        self.require_is_intermediated_farm(&farm_address);
        self.require_wrapped_farm_token_id_not_empty();
        self.require_wrapped_lp_token_id_not_empty();

        let payments_vec = self.call_value().all_esdt_transfers();
        let mut payments_iter = payments_vec.iter();
        let payment_0 = payments_iter
            .next()
            .unwrap_or_else(|| sc_panic!("bad payment len"));

        let payment_token_id = payment_0.token_identifier.clone();
        let payment_token_nonce = payment_0.token_nonce;
        let payment_amount = payment_0.amount;
        require!(payment_amount != 0u32, "Payment amount cannot be zero");

        let wrapped_farm_token = self.wrapped_farm_token().get_token_id();
        require!(
            payment_token_id == wrapped_farm_token,
            "Should only be used with wrapped farm tokens"
        );

        let wrapped_farm_token_attrs =
            self.get_wrapped_farm_token_attributes(&payment_token_id, payment_token_nonce);
        let farm_token_id = wrapped_farm_token_attrs.farm_token_id.clone();
        let farm_token_nonce = wrapped_farm_token_attrs.farm_token_nonce;
        let farm_amount = payment_amount.clone();

        let result = self.actual_compound_rewards(
            &farm_address,
            &farm_token_id,
            farm_token_nonce,
            &farm_amount,
        );

        let new_farm_token = result;
        let new_farm_token_id = new_farm_token.token_identifier;
        let new_farm_token_nonce = new_farm_token.token_nonce;
        let new_farm_token_amount = new_farm_token.amount;
        require!(
            new_farm_token_amount > 0u32,
            "Farm token amount received should be greater than 0"
        );

        let new_wrapped_farm_token_attributes = WrappedFarmTokenAttributes {
            farm_token_id: new_farm_token_id,
            farm_token_nonce: new_farm_token_nonce,
            farm_token_amount: new_farm_token_amount.clone(),
            farming_token_id: wrapped_farm_token_attrs.farming_token_id.clone(),
            farming_token_nonce: wrapped_farm_token_attrs.farming_token_nonce,
            farming_token_amount: self.rule_of_three_non_zero_result(
                &payment_amount,
                &wrapped_farm_token_attrs.farm_token_amount,
                &wrapped_farm_token_attrs.farming_token_amount,
            ),
        };
        let caller = self.blockchain().get_caller();
        let (new_wrapped_farm, created_with_merge) = self
            .create_wrapped_farm_tokens_by_merging_and_send(
                &new_wrapped_farm_token_attributes,
                &new_farm_token_amount,
                &farm_address,
                &caller,
                payments_iter,
            );
        self.send()
            .esdt_local_burn(&payment_token_id, payment_token_nonce, &payment_amount);

        self.emit_compound_rewards_farm_proxy_event(
            &caller,
            &farm_address,
            &payment_token_id,
            payment_token_nonce,
            &payment_amount,
            &new_wrapped_farm.token.token_identifier,
            new_wrapped_farm.token.token_nonce,
            &new_wrapped_farm.token.amount,
            &wrapped_farm_token_attrs,
            &new_wrapped_farm.attributes,
            created_with_merge,
        );

        new_wrapped_farm.token
    }

    fn create_wrapped_farm_tokens_by_merging_and_send(
        &self,
        attributes: &WrappedFarmTokenAttributes<Self::Api>,
        amount: &BigUint,
        farm_address: &ManagedAddress,
        caller: &ManagedAddress,
        additional_payments: ManagedVecRefIterator<Self::Api, EsdtTokenPayment<Self::Api>>,
    ) -> (WrappedFarmToken<Self::Api>, bool) {
        let wrapped_farm_token_id = self.wrapped_farm_token().get_token_id();
        self.merge_wrapped_farm_tokens_and_send(
            caller,
            farm_address,
            additional_payments,
            Option::Some(WrappedFarmToken {
                token: EsdtTokenPayment::new(wrapped_farm_token_id, 0, amount.clone()),
                attributes: attributes.clone(),
            }),
        )
    }

    fn actual_enter_farm(
        &self,
        farm_address: &ManagedAddress,
        farming_token_id: &TokenIdentifier,
        amount: &BigUint,
    ) -> EnterFarmResultType<Self::Api> {
        let asset_token_id = self.asset_token_id().get();
        if farming_token_id == &asset_token_id {
            self.send().esdt_local_mint(&asset_token_id, 0, amount);
        }

        let mut payments = ManagedVec::new();
        payments.push(EsdtTokenPayment::new(
            farming_token_id.clone(),
            0,
            amount.clone(),
        ));

        self.farm_contract_proxy(farm_address.clone())
            .enter_farm()
            .with_multi_token_transfer(payments)
            .execute_on_dest_context()
    }

    fn actual_exit_farm(
        &self,
        farm_address: &ManagedAddress,
        farm_token_id: &TokenIdentifier,
        farm_token_nonce: Nonce,
        amount: &BigUint,
    ) -> ExitFarmResultType<Self::Api> {
        self.farm_contract_proxy(farm_address.clone())
            .exit_farm()
            .add_esdt_token_transfer(farm_token_id.clone(), farm_token_nonce, amount.clone())
            .execute_on_dest_context()
    }

    fn actual_claim_rewards(
        &self,
        farm_address: &ManagedAddress,
        farm_token_id: &TokenIdentifier,
        farm_token_nonce: Nonce,
        amount: &BigUint,
    ) -> ClaimRewardsResultType<Self::Api> {
        let mut payments = ManagedVec::new();
        payments.push(EsdtTokenPayment::new(
            farm_token_id.clone(),
            farm_token_nonce,
            amount.clone(),
        ));

        self.farm_contract_proxy(farm_address.clone())
            .claim_rewards()
            .with_multi_token_transfer(payments)
            .execute_on_dest_context()
    }

    fn actual_compound_rewards(
        &self,
        farm_address: &ManagedAddress,
        farm_token_id: &TokenIdentifier,
        farm_token_nonce: Nonce,
        amount: &BigUint,
    ) -> CompoundRewardsResultType<Self::Api> {
        let mut payments = ManagedVec::new();
        payments.push(EsdtTokenPayment::new(
            farm_token_id.clone(),
            farm_token_nonce,
            amount.clone(),
        ));

        self.farm_contract_proxy(farm_address.clone())
            .compound_rewards()
            .with_multi_token_transfer(payments)
            .execute_on_dest_context()
    }

    fn require_is_intermediated_farm(&self, address: &ManagedAddress) {
        require!(
            self.intermediated_farms().contains(address),
            "Not an intermediated farm"
        );
    }

    fn require_wrapped_farm_token_id_not_empty(&self) {
        require!(!self.wrapped_farm_token().is_empty(), "Empty token id");
    }
}
