multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::PaymentsVec;
use fixed_supply_token::FixedSupplyToken;
use mergeable::ExternallyMergeable;
use multiversx_sc::api::{CallTypeApi, StorageMapperApi};

use crate::external_merging::merge_locked_tokens_through_factory;

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
pub struct WrappedLpTokenAttributes<M: ManagedTypeApi> {
    pub lp_token_id: TokenIdentifier<M>,
    pub lp_token_amount: BigUint<M>,
    pub locked_tokens: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> FixedSupplyToken<M> for WrappedLpTokenAttributes<M> {
    fn get_total_supply(&self) -> BigUint<M> {
        self.lp_token_amount.clone()
    }

    fn into_part(self, payment_amount: &BigUint<M>) -> Self {
        let new_lp_token_amount = payment_amount.clone();
        let new_locked_tokens_amount =
            self.rule_of_three_non_zero_result(payment_amount, &self.locked_tokens.amount);
        let new_locked_tokens = EsdtTokenPayment::new(
            self.locked_tokens.token_identifier,
            self.locked_tokens.token_nonce,
            new_locked_tokens_amount,
        );

        WrappedLpTokenAttributes {
            lp_token_id: self.lp_token_id,
            lp_token_amount: new_lp_token_amount,
            locked_tokens: new_locked_tokens,
        }
    }
}

impl<M: ManagedTypeApi> ExternallyMergeable<M> for WrappedLpTokenAttributes<M> {
    fn can_be_merged_externally_with(&self, other: &Self) -> bool {
        let same_lp_token = self.lp_token_id == other.lp_token_id;
        let same_locked_token_id =
            self.locked_tokens.token_identifier == other.locked_tokens.token_identifier;

        same_lp_token && same_locked_token_id
    }
}

#[derive(ManagedVecItem, Clone)]
pub struct WrappedLpToken<M: ManagedTypeApi> {
    pub payment: EsdtTokenPayment<M>,
    pub attributes: WrappedLpTokenAttributes<M>,
}

impl<M: ManagedTypeApi + StorageMapperApi + CallTypeApi> WrappedLpToken<M> {
    pub fn new_from_payments(
        payments: &PaymentsVec<M>,
        wrapped_token_mapper: &NonFungibleTokenMapper<M>,
    ) -> ManagedVec<M, Self> {
        wrapped_token_mapper.require_all_same_token(payments);

        let mut output = ManagedVec::new();
        for payment in payments {
            let attributes: WrappedLpTokenAttributes<M> =
                wrapped_token_mapper.get_token_attributes(payment.token_nonce);
            let wrapped_lp_token = WrappedLpToken {
                payment,
                attributes,
            };

            output.push(wrapped_lp_token);
        }

        output
    }
}

/// Merges all tokens under a single one, by also merging the locked tokens.
pub fn merge_wrapped_lp_tokens<M: CallTypeApi + StorageMapperApi>(
    original_caller: &ManagedAddress<M>,
    factory_address: ManagedAddress<M>,
    wrapped_lp_token_mapper: &NonFungibleTokenMapper<M>,
    mut wrapped_lp_tokens: ManagedVec<M, WrappedLpToken<M>>,
) -> WrappedLpToken<M> {
    let first_item = wrapped_lp_tokens.get(0);
    wrapped_lp_tokens.remove(0);

    let first_token_attributes = first_item.attributes.into_part(&first_item.payment.amount);

    let mut locked_tokens_to_merge =
        ManagedVec::from_single_item(first_token_attributes.locked_tokens.clone());
    let mut total_lp_tokens = first_token_attributes.lp_token_amount.clone();
    for wrapped_lp in &wrapped_lp_tokens {
        let attributes = wrapped_lp.attributes.into_part(&wrapped_lp.payment.amount);
        first_token_attributes.error_if_not_externally_mergeable(&attributes);

        total_lp_tokens += attributes.lp_token_amount;
        locked_tokens_to_merge.push(attributes.locked_tokens);
    }

    let new_locked_tokens = merge_locked_tokens_through_factory(
        original_caller,
        factory_address,
        locked_tokens_to_merge,
    );
    let new_wrapped_lp_token_attributes = WrappedLpTokenAttributes {
        lp_token_id: first_token_attributes.lp_token_id,
        lp_token_amount: total_lp_tokens,
        locked_tokens: new_locked_tokens,
    };
    let new_token_amount = new_wrapped_lp_token_attributes.get_total_supply();
    let new_tokens =
        wrapped_lp_token_mapper.nft_create(new_token_amount, &new_wrapped_lp_token_attributes);

    WrappedLpToken {
        payment: new_tokens,
        attributes: new_wrapped_lp_token_attributes,
    }
}
