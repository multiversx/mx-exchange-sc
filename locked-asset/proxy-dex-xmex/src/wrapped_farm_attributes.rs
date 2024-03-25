multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::PaymentsVec;
use fixed_supply_token::FixedSupplyToken;
use mergeable::ExternallyMergeable;
use multiversx_sc::api::{CallTypeApi, StorageMapperApi};

use crate::{
    external_merging::{merge_farm_tokens_through_farm, merge_locked_tokens_through_factory},
    wrapped_lp_attributes::{merge_wrapped_lp_tokens, WrappedLpToken},
};

#[derive(
    ManagedVecItem,
    TopEncode,
    TopDecode,
    NestedEncode,
    NestedDecode,
    TypeAbi,
    Clone,
    PartialEq,
    Debug,
)]
pub struct WrappedFarmTokenAttributes<M: ManagedTypeApi> {
    pub farm_token: EsdtTokenPayment<M>,
    pub proxy_farming_token: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> FixedSupplyToken<M> for WrappedFarmTokenAttributes<M> {
    fn get_total_supply(&self) -> BigUint<M> {
        self.farm_token.amount.clone()
    }

    fn into_part(self, payment_amount: &BigUint<M>) -> Self {
        let new_farm_token_amount = payment_amount.clone();
        let mut new_farm_tokens = self.farm_token.clone();
        new_farm_tokens.amount = new_farm_token_amount;

        let new_proxy_farming_token_amount =
            self.rule_of_three_non_zero_result(payment_amount, &self.proxy_farming_token.amount);
        let mut new_proxy_farming_tokens = self.proxy_farming_token;
        new_proxy_farming_tokens.amount = new_proxy_farming_token_amount;

        WrappedFarmTokenAttributes {
            farm_token: new_farm_tokens,
            proxy_farming_token: new_proxy_farming_tokens,
        }
    }
}

impl<M: ManagedTypeApi> ExternallyMergeable<M> for WrappedFarmTokenAttributes<M> {
    fn can_be_merged_externally_with(&self, other: &Self) -> bool {
        let same_farm_token = self.farm_token.token_identifier == other.farm_token.token_identifier;
        let same_proxy_farming_token =
            self.proxy_farming_token.token_identifier == other.proxy_farming_token.token_identifier;

        same_farm_token && same_proxy_farming_token
    }
}

#[derive(ManagedVecItem, Clone)]
pub struct WrappedFarmToken<M: ManagedTypeApi> {
    pub payment: EsdtTokenPayment<M>,
    pub attributes: WrappedFarmTokenAttributes<M>,
}

impl<M: ManagedTypeApi + StorageMapperApi + CallTypeApi> WrappedFarmToken<M> {
    pub fn new_from_payments(
        payments: &PaymentsVec<M>,
        wrapped_token_mapper: &NonFungibleTokenMapper<M>,
    ) -> ManagedVec<M, Self> {
        wrapped_token_mapper.require_all_same_token(payments);

        let mut output = ManagedVec::new();
        for payment in payments {
            let attributes: WrappedFarmTokenAttributes<M> =
                wrapped_token_mapper.get_token_attributes(payment.token_nonce);
            let wrapped_farm_token = WrappedFarmToken {
                payment,
                attributes,
            };

            output.push(wrapped_farm_token);
        }

        output
    }
}

/// Merges all wrapped farm tokens under a single one, by also merging the underlying
/// farm and locked tokens. Treats WrappedLp and LockedToken farms differently.
pub fn merge_wrapped_farm_tokens<M: CallTypeApi + StorageMapperApi>(
    original_caller: &ManagedAddress<M>,
    factory_address: ManagedAddress<M>,
    farm_address: ManagedAddress<M>,
    wrapped_lp_token_mapper: &NonFungibleTokenMapper<M>,
    wrapped_farm_token_mapper: &NonFungibleTokenMapper<M>,
    mut wrapped_farm_tokens: ManagedVec<M, WrappedFarmToken<M>>,
) -> WrappedFarmToken<M> {
    let first_item = wrapped_farm_tokens.get(0);
    wrapped_farm_tokens.remove(0);

    let first_token_attributes = first_item.attributes.into_part(&first_item.payment.amount);

    let mut farm_tokens_to_merge =
        ManagedVec::from_single_item(first_token_attributes.farm_token.clone());
    let mut farming_tokens_to_merge =
        ManagedVec::from_single_item(first_token_attributes.proxy_farming_token.clone());
    for wrapped_farm in &wrapped_farm_tokens {
        let attributes = wrapped_farm
            .attributes
            .into_part(&wrapped_farm.payment.amount);
        first_token_attributes.error_if_not_externally_mergeable(&attributes);

        farm_tokens_to_merge.push(attributes.farm_token);
        farming_tokens_to_merge.push(attributes.proxy_farming_token);
    }

    let wrapped_lp_token_id = wrapped_lp_token_mapper.get_token_id();
    let merged_farming_tokens = if first_token_attributes.proxy_farming_token.token_identifier
        == wrapped_lp_token_id
    {
        let wrapped_lp_tokens =
            WrappedLpToken::new_from_payments(&farming_tokens_to_merge, wrapped_lp_token_mapper);
        let merged_wrapped_lp_tokens = merge_wrapped_lp_tokens(
            original_caller,
            factory_address,
            wrapped_lp_token_mapper,
            wrapped_lp_tokens,
        );

        merged_wrapped_lp_tokens.payment
    } else {
        merge_locked_tokens_through_factory(
            original_caller,
            factory_address,
            farming_tokens_to_merge,
        )
    };

    let merged_farm_tokens =
        merge_farm_tokens_through_farm(original_caller, farm_address, farm_tokens_to_merge);
    let new_wrapped_farm_token_attributes = WrappedFarmTokenAttributes {
        farm_token: merged_farm_tokens,
        proxy_farming_token: merged_farming_tokens,
    };
    let new_token_amount = new_wrapped_farm_token_attributes.get_total_supply();
    let new_tokens =
        wrapped_farm_token_mapper.nft_create(new_token_amount, &new_wrapped_farm_token_attributes);

    WrappedFarmToken {
        payment: new_tokens,
        attributes: new_wrapped_farm_token_attributes,
    }
}
