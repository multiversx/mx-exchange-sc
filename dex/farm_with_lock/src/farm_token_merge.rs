elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::FarmTokenAttributes;
use farm_token::FarmToken;
use token_merge::ValueWeight;

use super::custom_config;

#[elrond_wasm::module]
pub trait FarmTokenMergeModule:
    token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + custom_config::CustomConfigModule
    + config::ConfigModule
    + token_merge::TokenMergeModule
{
    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<EsdtTokenPayment<Self::Api>> {
        let caller = self.blockchain().get_caller();
        let payments_vec = self.get_all_payments_managed_vec();
        let payments_iter = payments_vec.iter();

        let attrs = self.get_merged_farm_token_attributes(payments_iter.clone(), Option::None)?;
        let farm_token_id = self.farm_token_id().get();
        self.burn_farm_tokens_from_payments(payments_iter);

        let new_nonce = self.mint_farm_tokens(&farm_token_id, &attrs.current_farm_amount, &attrs);
        let new_amount = attrs.current_farm_amount;

        self.transfer_execute_custom(
            &caller,
            &farm_token_id,
            new_nonce,
            &new_amount,
            &opt_accept_funds_func,
        )?;

        Ok(self.create_payment(&farm_token_id, new_nonce, &new_amount))
    }

    fn get_merged_farm_token_attributes(
        &self,
        payments: ManagedVecIterator<EsdtTokenPayment<Self::Api>>,
        replic: Option<FarmToken<Self::Api>>,
    ) -> SCResult<FarmTokenAttributes<Self::Api>> {
        require!(
            !payments.is_empty() || replic.is_some(),
            "No tokens to merge"
        );

        let mut tokens = ManagedVec::new();
        let farm_token_id = self.farm_token_id().get();

        for payment in payments {
            require!(payment.amount != 0u64, "zero entry amount");
            require!(
                payment.token_identifier == farm_token_id,
                "Not a farm token"
            );

            tokens.push(FarmToken {
                token_amount: self.create_payment(
                    &payment.token_identifier,
                    payment.token_nonce,
                    &payment.amount,
                ),
                attributes: self
                    .get_farm_attributes(&payment.token_identifier, payment.token_nonce)?,
            });
        }

        if let Some(r) = replic {
            tokens.push(r);
        }

        if tokens.len() == 1 {
            if let Some(t) = tokens.get(0) {
                return Ok(t.attributes);
            }
        }

        let aggregated_attributes = FarmTokenAttributes {
            reward_per_share: self.aggregated_reward_per_share(&tokens),
            entering_epoch: self.blockchain().get_block_epoch(),
            original_entering_epoch: self.aggregated_original_entering_epoch(&tokens),
            initial_farming_amount: self.aggregated_initial_farming_amount(&tokens)?,
            compounded_reward: self.aggregated_compounded_reward(&tokens),
            current_farm_amount: self.aggregated_current_farm_amount(&tokens),
        };

        Ok(aggregated_attributes)
    }

    fn aggregated_reward_per_share(&self, tokens: &ManagedVec<FarmToken<Self::Api>>) -> BigUint {
        let mut dataset = ManagedVec::new();
        tokens.iter().for_each(|x| {
            dataset.push(ValueWeight {
                value: x.attributes.reward_per_share.clone(),
                weight: x.token_amount.amount.clone(),
            })
        });
        self.weighted_average_ceil(dataset)
    }

    fn aggregated_initial_farming_amount(
        &self,
        tokens: &ManagedVec<FarmToken<Self::Api>>,
    ) -> SCResult<BigUint> {
        let mut sum = BigUint::zero();
        for x in tokens.iter() {
            sum += &self.rule_of_three_non_zero_result(
                &x.token_amount.amount,
                &x.attributes.current_farm_amount,
                &x.attributes.initial_farming_amount,
            )?;
        }
        Ok(sum)
    }

    fn aggregated_compounded_reward(&self, tokens: &ManagedVec<FarmToken<Self::Api>>) -> BigUint {
        let mut sum = BigUint::zero();
        tokens.iter().for_each(|x| {
            sum += &self.rule_of_three(
                &x.token_amount.amount,
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
            .for_each(|x| aggregated_amount += &x.token_amount.amount);
        aggregated_amount
    }

    fn aggregated_original_entering_epoch(&self, tokens: &ManagedVec<FarmToken<Self::Api>>) -> u64 {
        let mut dataset = ManagedVec::new();
        tokens.iter().for_each(|x| {
            dataset.push(ValueWeight {
                value: BigUint::from(x.attributes.original_entering_epoch),
                weight: x.token_amount.amount.clone(),
            })
        });
        let avg = self.weighted_average(dataset);
        avg.to_u64().unwrap()
    }

    fn weighted_average(&self, dataset: ManagedVec<ValueWeight<Self::Api>>) -> BigUint {
        let mut weight_sum = BigUint::zero();
        dataset
            .iter()
            .for_each(|x| weight_sum = &weight_sum + &x.weight);

        let mut elem_weight_sum = BigUint::zero();
        dataset
            .iter()
            .for_each(|x| elem_weight_sum += &x.value * &x.weight);

        elem_weight_sum / weight_sum
    }
}
