#![no_std]
#![feature(generic_associated_types)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_errors::{
    ERROR_NOT_A_FARM_TOKEN, ERROR_NO_TOKEN_TO_MERGE, ERROR_TOO_MANY_ADDITIONAL_PAYMENTS,
    ERROR_ZERO_AMOUNT,
};
use common_structs::{mergeable_token_traits::*, FarmTokenAttributes, PaymentAttributesPair};
use token_merge_helper::{ValueWeight, WeightedAverageType};

pub const MAX_ADDITIONAL_TOKENS: usize = 10;
pub const MAX_TOTAL_TOKENS: usize = MAX_ADDITIONAL_TOKENS + 1;

#[elrond_wasm::module]
pub trait FarmTokenMergeModule:
    token_merge_helper::TokenMergeHelperModule
    + farm_token::FarmTokenModule
    + admin_whitelist::AdminWhitelistModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn create_farm_tokens_by_merging(
        &self,
        virtual_position: FarmToken<Self::Api>,
        additional_positions: &ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> (FarmToken<Self::Api>, bool) {
        let farm_token_id = virtual_position.payment.token_identifier.clone();
        let additional_payments_len = additional_positions.len();
        let merged_attributes =
            self.get_merged_farm_token_attributes(additional_positions, Some(virtual_position));

        self.burn_farm_tokens_from_payments(additional_positions);

        let new_amount = merged_attributes.current_farm_amount.clone();
        let new_tokens = self.mint_farm_tokens(farm_token_id, new_amount, &merged_attributes);

        let new_farm_token = FarmToken {
            payment: new_tokens,
            attributes: merged_attributes,
        };
        let is_merged = additional_payments_len != 0;

        (new_farm_token, is_merged)
    }

    fn get_default_merged_farm_token_attributes(
        &self,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        virtual_position: Option<PaymentAttributesPair<Self::Api, FarmTokenAttributes<Self::Api>>>,
    ) -> FarmTokenAttributes<Self::Api> {
        require!(
            !payments.is_empty() || virtual_position.is_some(),
            ERROR_NO_TOKEN_TO_MERGE
        );
        require!(
            payments.len() <= MAX_ADDITIONAL_TOKENS,
            ERROR_TOO_MANY_ADDITIONAL_PAYMENTS
        );

        let mut tokens = ArrayVec::<
            PaymentAttributesPair<Self::Api, FarmTokenAttributes<Self::Api>>,
            MAX_TOTAL_TOKENS,
        >::new();
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
            original_entering_epoch: current_epoch,
            initial_farming_amount: self.aggregated_initial_farming_amount(&tokens),
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
                value: token.get_payment_amount().clone(),
                weight: token.get_reward_per_share().clone(),
            })
        }

        self.weighted_average(dataset, WeightedAverageType::Ceil)
    }

    fn aggregated_initial_farming_amount<
        T: PaymentAmountGetter<Self::Api>
            + CurrentFarmAmountGetter<Self::Api>
            + InitialFarmingAmountGetter<Self::Api>,
    >(
        &self,
        tokens: &ArrayVec<T, MAX_TOTAL_TOKENS>,
    ) -> BigUint {
        let mut sum = BigUint::zero();
        for token in tokens {
            sum += self.rule_of_three_non_zero_result(
                token.get_payment_amount(),
                token.get_current_farm_amount(),
                token.get_initial_farming_amount(),
            );
        }
        sum
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