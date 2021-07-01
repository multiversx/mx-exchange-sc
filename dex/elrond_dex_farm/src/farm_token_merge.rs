elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use farm_token::{FarmToken, FarmTokenAttributes};
use token_merge::{ValueWeight};

use super::config;

use super::farm_token;

#[elrond_wasm_derive::module]
pub trait FarmTokenMergeModule:
    nft_deposit::NftDepositModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + token_supply::TokenSupplyModule
    + config::ConfigModule
    + token_merge::TokenMergeModule
{
    #[endpoint(mergeTokens)]
    fn merge_tokens(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let attrs = self.get_merged_farm_token_attributes(&caller, Option::None)?;
        let farm_token_id = self.farm_token_id().get();

        self.burn_merge_tokens(&caller);
        self.nft_deposit(&caller).clear();

        self.nft_create_tokens(&farm_token_id, &attrs.current_farm_amount, &attrs);
        self.increase_nonce();

        self.send_nft_tokens(
            &farm_token_id,
            self.farm_token_nonce().get(),
            &attrs.current_farm_amount,
            &caller,
            &opt_accept_funds_func,
        );

        Ok(())
    }

    fn get_merged_farm_token_attributes(
        &self,
        caller: &Address,
        replic: Option<FarmToken<Self::BigUint>>,
    ) -> SCResult<FarmTokenAttributes<Self::BigUint>> {
        let deposit_len = self.nft_deposit(caller).len();
        require!(deposit_len != 0 || replic.is_some(), "No tokens to merge");

        let mut index = 1;
        let mut tokens = Vec::new();
        let farm_token_id = self.farm_token_id().get();

        while index <= deposit_len {
            let entry = self.nft_deposit(caller).get(index);
            require!(entry.token_id == farm_token_id, "Not a farm token");

            tokens.push(FarmToken {
                token_amount: entry.clone(),
                attributes: self.get_farm_attributes(&entry.token_id, entry.token_nonce)?,
            });

            index += 1;
        }

        if replic.is_some() {
            tokens.push(replic.unwrap());
        }

        if tokens.len() == 1 {
            return Ok(tokens[0].clone().attributes);
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
        let avg = self.weighted_average(dataset);
        avg.to_u64().unwrap()
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
}
