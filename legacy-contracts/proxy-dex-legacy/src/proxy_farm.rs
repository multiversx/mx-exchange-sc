#![allow(clippy::too_many_arguments)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Nonce;
use common_structs::{RawResultWrapper, RawResultsType};
use factory::attr_ex_helper;

use crate::energy_update;
use crate::proxy_common::WrappedFarmTokenAttributes;

use super::events;
use super::proxy_common;
use super::proxy_pair;

type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

mod farm_proxy {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait FarmProxy {
        #[payable("*")]
        #[endpoint(exitFarm)]
        fn exit_farm(&self) -> super::ExitFarmResultType<Self::Api>;
    }
}

#[derive(ManagedVecItem, Clone)]
pub struct WrappedFarmToken<M: ManagedTypeApi> {
    pub token_amount: EsdtTokenPayment<M>,
    pub attributes: WrappedFarmTokenAttributes<M>,
}

#[multiversx_sc::module]
pub trait ProxyFarmModule:
    proxy_common::ProxyCommonModule
    + proxy_pair::ProxyPairModule
    + token_merge_helper::TokenMergeHelperModule
    + events::EventsModule
    + energy_update::EnergyUpdateModule
    + attr_ex_helper::AttrExHelper
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
    #[endpoint(exitFarmProxy)]
    fn exit_farm_proxy(&self, farm_address: &ManagedAddress) {
        self.require_is_intermediated_farm(farm_address);
        self.require_wrapped_farm_token_id_not_empty();
        self.require_wrapped_lp_token_id_not_empty();

        let (token_id, token_nonce, amount) = self.call_value().single_esdt().into_tuple();

        require!(amount != 0, "Payment amount cannot be zero");
        require!(
            token_id == self.wrapped_farm_token_id().get(),
            "Should only be used with wrapped farm tokens"
        );

        let wrapped_farm_token_attrs =
            self.get_wrapped_farm_token_attributes(&token_id, token_nonce);
        let farm_token_id = wrapped_farm_token_attrs.farm_token_id.clone();
        let farm_token_nonce = wrapped_farm_token_attrs.farm_token_nonce;

        let farm_result = self
            .actual_exit_farm(farm_address, &farm_token_id, farm_token_nonce, &amount)
            .into_tuple();
        let farming_token_returned = farm_result.0;
        let reward_token_returned = farm_result.1;

        let caller = self.blockchain().get_caller();
        let mut payments_vec = ManagedVec::new();
        payments_vec.push(EsdtTokenPayment::new(
            wrapped_farm_token_attrs.farming_token_id.clone(),
            wrapped_farm_token_attrs.farming_token_nonce,
            farming_token_returned.amount.clone(),
        ));
        payments_vec.push(EsdtTokenPayment::new(
            reward_token_returned.token_identifier.clone(),
            reward_token_returned.token_nonce,
            reward_token_returned.amount.clone(),
        ));
        self.send_multiple_tokens_if_not_zero(&caller, &payments_vec);
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
            farm_address,
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
    }

    fn actual_exit_farm(
        &self,
        farm_address: &ManagedAddress,
        farm_token_id: &TokenIdentifier,
        farm_token_nonce: Nonce,
        amount: &BigUint,
    ) -> ExitFarmResultType<Self::Api> {
        let raw_results: RawResultsType<Self::Api> = self
            .farm_contract_proxy(farm_address.clone())
            .exit_farm()
            .with_esdt_transfer((farm_token_id.clone(), farm_token_nonce, amount.clone()))
            .execute_on_dest_context();

        let mut results_wrapper = RawResultWrapper::new(raw_results);
        results_wrapper.trim_results_front(2);

        let farming_tokens = results_wrapper.decode_next_result();
        let reward_tokens = results_wrapper.decode_next_result();

        (farming_tokens, reward_tokens).into()
    }

    fn require_is_intermediated_farm(&self, address: &ManagedAddress) {
        require!(
            self.intermediated_farms().contains(address),
            "Not an intermediated farm"
        );
    }

    fn require_wrapped_farm_token_id_not_empty(&self) {
        require!(!self.wrapped_farm_token_id().is_empty(), "Empty token id");
    }

    #[proxy]
    fn farm_contract_proxy(&self, to: ManagedAddress) -> farm_proxy::Proxy<Self::Api>;
}
