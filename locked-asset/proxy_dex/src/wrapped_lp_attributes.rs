elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::PaymentsVec;
use elrond_wasm::api::{CallTypeApi, StorageMapperApi};
use fixed_supply_token::FixedSupplyToken;
use mergeable::Mergeable;

use crate::proxy_common::FACTORY_MERGE_TOKENS_ENDPOINT_NAME;

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct WrappedLpTokenAttributes<M: ManagedTypeApi> {
    pub lp_token_id: TokenIdentifier<M>,
    pub lp_token_amount: BigUint<M>,
    pub locked_tokens: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> FixedSupplyToken<M> for WrappedLpTokenAttributes<M> {
    fn get_total_supply(&self) -> &BigUint<M> {
        &self.lp_token_amount
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

impl<M: ManagedTypeApi> Mergeable<M> for WrappedLpTokenAttributes<M> {
    /// locked token nonce can differ, since they get merged through another call
    fn can_merge_with(&self, other: &Self) -> bool {
        let same_lp_token = self.lp_token_id == other.lp_token_id;
        let same_locked_token_id =
            self.locked_tokens.token_identifier == other.locked_tokens.token_identifier;

        same_lp_token && same_locked_token_id
    }

    fn merge_with(&mut self, other: Self) {
        self.error_if_not_mergeable(&other);

        self.lp_token_amount += other.lp_token_amount;
        self.locked_tokens.amount += other.locked_tokens.amount;
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
/// The caller function should handle the minting of the new wrapped LP tokens.
pub fn merge_wrapped_lp_tokens_through_factory<M: CallTypeApi>(
    factory_address: ManagedAddress<M>,
    mut wrapped_lp_tokens: ManagedVec<M, WrappedLpToken<M>>,
) -> WrappedLpTokenAttributes<M> {
    let first_item = wrapped_lp_tokens.get(0);
    wrapped_lp_tokens.remove(0);

    let mut merged_wrapped_lp_attributes =
        first_item.attributes.into_part(&first_item.payment.amount);
    let mut locked_tokens_to_merge = ManagedVec::new();
    for wrapped_lp in &wrapped_lp_tokens {
        let attributes = wrapped_lp.attributes.into_part(&wrapped_lp.payment.amount);
        locked_tokens_to_merge.push(attributes.locked_tokens.clone());

        merged_wrapped_lp_attributes.merge_with(attributes);
    }

    let merge_endpoint_name = ManagedBuffer::new_from_bytes(FACTORY_MERGE_TOKENS_ENDPOINT_NAME);
    let contract_call = ContractCall::<M, EsdtTokenPayment<M>>::new_with_esdt_payment(
        factory_address,
        merge_endpoint_name,
        locked_tokens_to_merge,
    );
    let new_locked_tokens = contract_call.execute_on_dest_context();
    merged_wrapped_lp_attributes.locked_tokens = new_locked_tokens;

    merged_wrapped_lp_attributes
}
