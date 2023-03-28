#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_errors::{
    ERROR_NOT_A_FARM_TOKEN, ERROR_NO_TOKEN_TO_MERGE, ERROR_TOO_MANY_ADDITIONAL_PAYMENTS,
    ERROR_ZERO_AMOUNT,
};
use common_structs::{
    mergeable_token_traits::*, DefaultFarmPaymentAttributesPair, FarmTokenAttributes,
    PaymentAttributesPair,
};
use token_merge_helper::{ValueWeight, WeightedAverageType};

pub const MAX_ADDITIONAL_TOKENS: usize = 10;
pub const MAX_TOTAL_TOKENS: usize = MAX_ADDITIONAL_TOKENS + 1;

#[multiversx_sc::module]
pub trait FarmTokenMergeModule:
    token_merge_helper::TokenMergeHelperModule
    + farm_token::FarmTokenModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn create_farm_tokens_by_merging<AttributesType, AttributesMergingFunction>(
        &self,
        virtual_position: PaymentAttributesPair<Self::Api, AttributesType>,
        additional_farm_tokens: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        attributes_merging_fn: AttributesMergingFunction,
    ) -> PaymentAttributesPair<Self::Api, AttributesType>
    where
        AttributesType: Clone
            + TopEncode
            + TopDecode
            + NestedEncode
            + NestedDecode
            + CurrentFarmAmountGetter<Self::Api>,
        AttributesMergingFunction: Fn(
            &Self,
            &ManagedVec<EsdtTokenPayment<Self::Api>>,
            Option<PaymentAttributesPair<Self::Api, AttributesType>>,
        ) -> AttributesType,
    {
        let farm_token_id = virtual_position.payment.token_identifier.clone();
        let merged_attributes =
            attributes_merging_fn(self, additional_farm_tokens, Some(virtual_position));

        self.burn_farm_tokens_from_payments(additional_farm_tokens);

        let new_amount = merged_attributes.get_current_farm_amount().clone();
        let new_tokens = self.mint_farm_tokens(farm_token_id, new_amount, &merged_attributes);

        PaymentAttributesPair {
            payment: new_tokens,
            attributes: merged_attributes,
        }
    }

    fn get_default_merged_farm_token_attributes(
        &self,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        virtual_position: Option<DefaultFarmPaymentAttributesPair<Self::Api>>,
    ) -> FarmTokenAttributes<Self::Api> {
        require!(
            !payments.is_empty() || virtual_position.is_some(),
            ERROR_NO_TOKEN_TO_MERGE
        );
        require!(
            payments.len() <= MAX_ADDITIONAL_TOKENS,
            ERROR_TOO_MANY_ADDITIONAL_PAYMENTS
        );

        let mut tokens =
            ArrayVec::<DefaultFarmPaymentAttributesPair<Self::Api>, MAX_TOTAL_TOKENS>::new();
        let farm_token_id = self.farm_token().get_token_id();

        for payment in payments {
            require!(payment.amount != 0u64, ERROR_ZERO_AMOUNT);
            require!(
                payment.token_identifier == farm_token_id,
                ERROR_NOT_A_FARM_TOKEN
            );

            let attributes =
                self.get_farm_token_attributes(&payment.token_identifier, payment.token_nonce);
            unsafe {
                tokens.push_unchecked(PaymentAttributesPair {
                    payment,
                    attributes,
                });
            }
        }

        if let Some(pos) = virtual_position {
            unsafe {
                tokens.push_unchecked(pos);
            }
        }

        if tokens.len() == 1 {
            return unsafe { tokens.get_unchecked(0).attributes.clone() };
        }

        let current_epoch = self.blockchain().get_block_epoch();
        FarmTokenAttributes {
            reward_per_share: self.aggregated_reward_per_share(&tokens),
            entering_epoch: current_epoch,
            compounded_reward: self.aggregated_compounded_reward(&tokens),
            current_farm_amount: self.aggregated_current_farm_amount(&tokens),
        }
    }

    fn aggregated_reward_per_share<
        T: PaymentAmountGetter<Self::Api> + RewardPerShareGetter<Self::Api>,
    >(
        &self,
        tokens: &ArrayVec<T, MAX_TOTAL_TOKENS>,
    ) -> BigUint {
        let mut dataset = ManagedVec::new();
        for token in tokens {
            dataset.push(ValueWeight {
                value: token.get_reward_per_share().clone(),
                weight: token.get_payment_amount().clone(),
            })
        }

        self.weighted_average(dataset, WeightedAverageType::Ceil)
    }

    fn aggregated_compounded_reward<
        T: PaymentAmountGetter<Self::Api>
            + CurrentFarmAmountGetter<Self::Api>
            + CompoundedRewardAmountGetter<Self::Api>,
    >(
        &self,
        tokens: &ArrayVec<T, MAX_TOTAL_TOKENS>,
    ) -> BigUint {
        let mut sum = BigUint::zero();
        for token in tokens {
            sum += self.rule_of_three(
                token.get_payment_amount(),
                token.get_current_farm_amount(),
                token.get_compounded_reward_amount(),
            )
        }

        sum
    }

    fn aggregated_current_farm_amount<T: PaymentAmountGetter<Self::Api>>(
        &self,
        tokens: &ArrayVec<T, MAX_TOTAL_TOKENS>,
    ) -> BigUint {
        let mut aggregated_amount = BigUint::zero();
        for token in tokens {
            aggregated_amount += token.get_payment_amount()
        }

        aggregated_amount
    }
}
