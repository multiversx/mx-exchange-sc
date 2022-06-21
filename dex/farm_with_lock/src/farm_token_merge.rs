elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_errors::*;

use common_structs::FarmTokenAttributes;
use farm_token::FarmToken;
use token_merge::ValueWeight;

#[elrond_wasm::module]
pub trait FarmTokenMergeModule:
    token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + config::ConfigModule
    + token_merge::TokenMergeModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens(&self) -> EsdtTokenPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payments = self.call_value().all_esdt_transfers();

        let attrs = self.get_merged_farm_token_attributes(&payments, Option::None);
        let farm_token_id = self.farm_token().get_token_id();
        self.burn_farm_tokens_from_payments(&payments);

        let new_tokens =
            self.mint_farm_tokens(farm_token_id, attrs.current_farm_amount.clone(), &attrs);

        self.send().direct_esdt(
            &caller,
            &new_tokens.token_identifier,
            new_tokens.token_nonce,
            &new_tokens.amount,
        );

        new_tokens
    }

    fn get_merged_farm_token_attributes(
        &self,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        replic: Option<&FarmToken<Self::Api>>,
    ) -> FarmTokenAttributes<Self::Api> {
        require!(
            !payments.is_empty() || replic.is_some(),
            ERROR_NO_TOKEN_TO_MERGE
        );

        let mut tokens = ManagedVec::new();
        let farm_token_id = self.farm_token().get_token_id();

        for payment in payments.iter() {
            require!(payment.amount != 0u64, ERROR_ZERO_AMOUNT);
            require!(
                payment.token_identifier == farm_token_id,
                ERROR_NOT_A_FARM_TOKEN
            );

            let attributes =
                self.get_farm_token_attributes(&payment.token_identifier, payment.token_nonce);
            tokens.push(FarmToken {
                payment,
                attributes,
            });
        }

        if let Some(r) = replic {
            tokens.push(r.clone());
        }

        if tokens.len() == 1 {
            if let Some(t) = tokens.try_get(0) {
                return t.attributes;
            }
        }

        FarmTokenAttributes {
            reward_per_share: self.aggregated_reward_per_share(&tokens),
            entering_epoch: self.blockchain().get_block_epoch(),
            original_entering_epoch: self.aggregated_original_entering_epoch(&tokens),
            initial_farming_amount: self.aggregated_initial_farming_amount(&tokens),
            compounded_reward: self.aggregated_compounded_reward(&tokens),
            current_farm_amount: self.aggregated_current_farm_amount(&tokens),
        }
    }

    fn aggregated_reward_per_share(&self, tokens: &ManagedVec<FarmToken<Self::Api>>) -> BigUint {
        let mut dataset = ManagedVec::new();
        tokens.iter().for_each(|x| {
            dataset.push(ValueWeight {
                value: x.attributes.reward_per_share.clone(),
                weight: x.payment.amount,
            })
        });
        self.weighted_average_ceil(dataset)
    }

    fn aggregated_initial_farming_amount(
        &self,
        tokens: &ManagedVec<FarmToken<Self::Api>>,
    ) -> BigUint {
        let mut sum = BigUint::zero();
        for x in tokens.iter() {
            sum += &self.rule_of_three_non_zero_result(
                &x.payment.amount,
                &x.attributes.current_farm_amount,
                &x.attributes.initial_farming_amount,
            );
        }
        sum
    }

    fn aggregated_compounded_reward(&self, tokens: &ManagedVec<FarmToken<Self::Api>>) -> BigUint {
        let mut sum = BigUint::zero();
        tokens.iter().for_each(|x| {
            sum += &self.rule_of_three(
                &x.payment.amount,
                &x.attributes.current_farm_amount,
                &x.attributes.compounded_reward,
            )
        });
        sum
    }

    fn aggregated_current_farm_amount(&self, tokens: &ManagedVec<FarmToken<Self::Api>>) -> BigUint {
        let mut aggregated_amount = BigUint::zero();
        tokens
            .iter()
            .for_each(|x| aggregated_amount += &x.payment.amount);
        aggregated_amount
    }

    fn aggregated_original_entering_epoch(&self, tokens: &ManagedVec<FarmToken<Self::Api>>) -> u64 {
        let mut dataset = ManagedVec::new();
        tokens.iter().for_each(|x| {
            dataset.push(ValueWeight {
                value: BigUint::from(x.attributes.original_entering_epoch),
                weight: x.payment.amount,
            })
        });
        let avg = self.weighted_average(dataset);
        avg.to_u64().unwrap()
    }
}
