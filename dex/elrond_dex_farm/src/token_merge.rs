elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use farm_token::{FarmToken, FarmTokenAttributes};

use super::config;

use super::farm_token;

#[derive(Clone, Copy)]
pub struct ValueWeight<BigUint: BigUintApi> {
    value: BigUint,
    weight: BigUint,
}

#[elrond_wasm_derive::module]
pub trait TokenMergeModule:
    nft_deposit::NftDepositModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + token_supply::TokenSupplyModule
    + config::ConfigModule
{
    fn get_merged_farm_token_attributes(&self) -> SCResult<FarmTokenAttributes<Self::BigUint>> {
        let caller = self.blockchain().get_caller();
        let deposit_len = self.nft_deposit(&caller).len();
        require!(deposit_len != 0, "No tokens to merge");

        let mut index = 1;
        let mut tokens = Vec::new();
        let farm_token_id = self.farm_token_id().get();

        while index <= deposit_len {
            let entry = self.nft_deposit(&caller).get(index);
            require!(entry.token_id == farm_token_id, "Not a farm token");

            tokens.push(FarmToken {
                token_amount: entry.clone(),
                attributes: self.get_farm_attributes(&entry.token_id, entry.token_nonce)?,
            });

            index += 1;
        }

        let aggregated_attributes = FarmTokenAttributes {
            reward_per_share: self.aggregated_reward_per_share(&tokens),
            entering_epoch: self.aggregated_entering_epoch(&tokens),
            apr_multiplier: self.aggregated_apr_multiplier(&tokens)?,
            with_locked_rewards: self.aggregated_with_lock_rewards(&tokens)?,
            initial_farming_amount: self.aggregated_initial_farming_amount(&tokens),
            compounded_reward: self.aggregated_compounded_reward(&tokens),
            current_farm_amount: self.aggregated_current_farm_amount(&tokens),
        };

        Ok(aggregated_attributes)
    }

    fn aggregated_reward_per_share(&self, tokens: &[FarmToken<Self::BigUint>]) -> Self::BigUint {
        let mut dataset = Vec::new();
        tokens.iter().for_each(|x| {
            dataset.push(ValueWeight {
                value: x.attributes.reward_per_share.clone(),
                weight: x.token_amount.amount.clone(),
            })
        });
        self.weighted_average_ceil(dataset)
    }

    fn aggregated_entering_epoch(&self, tokens: &[FarmToken<Self::BigUint>]) -> u64 {
        let mut dataset = Vec::new();
        tokens.iter().for_each(|x| {
            dataset.push(ValueWeight {
                value: Self::BigUint::from(x.attributes.entering_epoch),
                weight: x.token_amount.amount.clone(),
            })
        });
        let _avg = self.weighted_average(dataset);
        0 //TODO: update this after framework update
    }

    fn aggregated_apr_multiplier(&self, tokens: &[FarmToken<Self::BigUint>]) -> SCResult<u8> {
        let first_elem_value = tokens.get(1).unwrap().attributes.apr_multiplier;
        let mut same_value = true;
        tokens
            .iter()
            .for_each(|x| same_value &= first_elem_value == x.attributes.apr_multiplier);
        require!(same_value, "Cannot compute apr multiplier aggregate");
        Ok(first_elem_value)
    }

    fn aggregated_with_lock_rewards(&self, tokens: &[FarmToken<Self::BigUint>]) -> SCResult<bool> {
        let first_elem_value = tokens.get(1).unwrap().attributes.with_locked_rewards;
        let mut same_value = true;
        tokens
            .iter()
            .for_each(|x| same_value &= first_elem_value == x.attributes.with_locked_rewards);
        require!(same_value, "Cannot compute with locked rewards aggregate");
        Ok(first_elem_value)
    }

    fn aggregated_initial_farming_amount(
        &self,
        tokens: &[FarmToken<Self::BigUint>],
    ) -> Self::BigUint {
        let mut dataset = Vec::new();
        tokens.iter().for_each(|x| {
            dataset.push(ValueWeight {
                value: self.rule_of_three(
                    &x.token_amount.amount,
                    &x.attributes.current_farm_amount,
                    &x.attributes.initial_farming_amount,
                ),
                weight: x.token_amount.amount.clone(),
            })
        });
        self.weighted_average(dataset)
    }

    fn aggregated_compounded_reward(&self, tokens: &[FarmToken<Self::BigUint>]) -> Self::BigUint {
        let mut dataset = Vec::new();
        tokens.iter().for_each(|x| {
            dataset.push(ValueWeight {
                value: self.rule_of_three(
                    &x.token_amount.amount,
                    &x.attributes.current_farm_amount,
                    &x.attributes.compounded_reward,
                ),
                weight: x.token_amount.amount.clone(),
            })
        });
        self.weighted_average(dataset)
    }

    fn aggregated_current_farm_amount(&self, tokens: &[FarmToken<Self::BigUint>]) -> Self::BigUint {
        let mut aggregated_amount = Self::BigUint::zero();
        tokens
            .iter()
            .for_each(|x| aggregated_amount += &x.token_amount.amount);
        aggregated_amount
    }

    fn rule_of_three(
        &self,
        part: &Self::BigUint,
        total: &Self::BigUint,
        value: &Self::BigUint,
    ) -> Self::BigUint {
        &(part * value) / total
    }

    fn weighted_average(&self, dataset: Vec<ValueWeight<Self::BigUint>>) -> Self::BigUint {
        let mut weight_sum = Self::BigUint::zero();
        dataset
            .iter()
            .for_each(|x| weight_sum = &weight_sum + &x.weight);

        let mut elem_weight_sum = Self::BigUint::zero();
        dataset
            .iter()
            .for_each(|x| elem_weight_sum = &weight_sum + &(&x.value * &x.weight));

        elem_weight_sum / weight_sum
    }

    fn weighted_average_ceil(&self, dataset: Vec<ValueWeight<Self::BigUint>>) -> Self::BigUint {
        let mut weight_sum = Self::BigUint::zero();
        dataset
            .iter()
            .for_each(|x| weight_sum = &weight_sum + &x.weight);

        let mut elem_weight_sum = Self::BigUint::zero();
        dataset
            .iter()
            .for_each(|x| elem_weight_sum = &weight_sum + &(&x.value * &x.weight));

        (&elem_weight_sum + &weight_sum - Self::BigUint::from(1u64)) / weight_sum
    }
}
