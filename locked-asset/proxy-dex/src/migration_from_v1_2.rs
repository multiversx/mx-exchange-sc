elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::WrappedFarmTokenAttributes;

use super::events;
use super::proxy_common;
use super::proxy_pair;
use super::wrapped_farm_token_merge;
use super::wrapped_lp_token_merge;
use crate::proxy_farm;

mod farm_v1_2_contract_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait Farm {
        #[payable("*")]
        #[endpoint(migrateToNewFarm)]
        fn migrate_to_new_farm(&self, orig_caller: ManagedAddress) -> EsdtTokenPayment<Self::Api>;
    }
}

#[elrond_wasm::module]
pub trait MigrationModule:
    proxy_farm::ProxyFarmModule
    + proxy_common::ProxyCommonModule
    + proxy_pair::ProxyPairModule
    + token_merge::TokenMergeModule
    + token_send::TokenSendModule
    + wrapped_farm_token_merge::WrappedFarmTokenMerge
    + wrapped_lp_token_merge::WrappedLpTokenMerge
    + events::EventsModule
{
    #[payable("*")]
    #[endpoint(migrateV1_2Position)]
    fn migrate_v1_2_position(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_nonce] token_nonce: u64,
        #[payment_amount] amount: BigUint,
        farm_address: ManagedAddress,
    ) {
        self.require_is_intermediated_farm(&farm_address);
        self.require_wrapped_farm_token_id_not_empty();
        self.require_wrapped_lp_token_id_not_empty();

        let payments_vec = self.call_value().all_esdt_transfers();
        let mut payments_iter = payments_vec.iter();
        let payment_0 = payments_iter.next().unwrap();

        let payment_token_id = payment_0.token_identifier.clone();
        let payment_token_nonce = payment_0.token_nonce;
        let payment_amount = payment_0.amount;
        require!(payment_amount != 0u64, "Payment amount cannot be zero");

        let wrapped_farm_token = self.wrapped_farm_token_id().get();
        require!(
            payment_token_id == wrapped_farm_token,
            "Should only be used with wrapped farm tokens"
        );

        // The actual work starts here
        let wrapped_farm_token_attrs =
            self.get_wrapped_farm_token_attributes(&payment_token_id, payment_token_nonce);
        let farm_token_id = wrapped_farm_token_attrs.farm_token_id.clone();
        let farm_token_nonce = wrapped_farm_token_attrs.farm_token_nonce;
        let farm_amount = payment_amount;

        // Get the new farm position from the new contract.
        let new_pos = self
            .farm_v1_2_contract_proxy(farm_address)
            .migrate_to_new_farm(self.blockchain().get_sc_address())
            .add_token_transfer(farm_token_id, farm_token_nonce, farm_amount)
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after));

        // Burn the old proxy farm position
        self.send().esdt_local_burn(&token_id, token_nonce, &amount);

        // Create a new proxy farm position based on the new farm position.
        let new_attrs = WrappedFarmTokenAttributes {
            farm_token_id: new_pos.token_identifier.clone(),
            farm_token_nonce: new_pos.token_nonce,
            farm_token_amount: new_pos.amount.clone(),
            farming_token_id: wrapped_farm_token_attrs.farming_token_id,
            farming_token_nonce: wrapped_farm_token_attrs.farming_token_nonce,
            farming_token_amount: wrapped_farm_token_attrs.farming_token_amount,
        };
        let new_nonce = self.nft_create_tokens(&wrapped_farm_token, &new_pos.amount, &new_attrs);
        self.transfer_execute_custom(
            &self.blockchain().get_caller(),
            &wrapped_farm_token,
            new_nonce,
            &new_pos.amount,
            &OptionalArg::None,
        );
    }

    #[proxy]
    fn farm_v1_2_contract_proxy(
        &self,
        to: ManagedAddress,
    ) -> farm_v1_2_contract_proxy::Proxy<Self::Api>;
}
