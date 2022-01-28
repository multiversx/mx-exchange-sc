#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

pub mod custom_rewards;
pub mod farm_token_merge;
pub mod whitelist;

use common_structs::Nonce;
use config::State;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::farm_token_merge::StakingFarmTokenAttributes;
use config::{
    DEFAULT_BURN_GAS_LIMIT, DEFAULT_MINUMUM_FARMING_EPOCHS, DEFAULT_PENALTY_PERCENT,
    DEFAULT_TRANSFER_EXEC_GAS_LIMIT, MAX_PERCENT,
};
use farm_token_merge::StakingFarmToken;

pub type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
pub type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
pub type ClaimRewardsResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
pub type ExitFarmResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
pub type UnbondFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct UnbondSftAttributes {
    pub unlock_epoch: u64,
}

#[elrond_wasm::contract]
pub trait Farm:
    custom_rewards::CustomRewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
    + farm_token::FarmTokenModule
    + farm_token_merge::FarmTokenMergeModule
    + whitelist::WhitelistModule
{
    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[init]
    fn init(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        pair_contract_address: ManagedAddress,
        max_apr: BigUint,
        min_unbond_epochs: u64,
    ) -> SCResult<()> {
        require!(
            reward_token_id.is_esdt(),
            "Reward token ID is not a valid esdt identifier"
        );
        require!(
            farming_token_id.is_esdt(),
            "Farming token ID is not a valid esdt identifier"
        );
        require!(
            division_safety_constant != 0,
            "Division constant cannot be 0"
        );
        let farm_token = self.farm_token_id().get();
        require!(
            reward_token_id != farm_token,
            "Reward token ID cannot be farm token ID"
        );
        require!(
            farming_token_id != farm_token,
            "Farming token ID cannot be farm token ID"
        );
        require!(
            max_apr > 0 && max_apr < MAX_PERCENT,
            "Invalid max APR percentage"
        );

        self.state().set(&State::Inactive);
        self.penalty_percent()
            .set_if_empty(&DEFAULT_PENALTY_PERCENT);
        self.minimum_farming_epochs()
            .set_if_empty(&DEFAULT_MINUMUM_FARMING_EPOCHS);
        self.transfer_exec_gas_limit()
            .set_if_empty(&DEFAULT_TRANSFER_EXEC_GAS_LIMIT);
        self.burn_gas_limit().set_if_empty(&DEFAULT_BURN_GAS_LIMIT);
        self.division_safety_constant()
            .set_if_empty(&division_safety_constant);

        self.owner().set(&self.blockchain().get_caller());
        self.reward_token_id().set(&reward_token_id);
        self.farming_token_id().set(&farming_token_id);
        self.pair_contract_address().set(&pair_contract_address);
        self.max_annual_percentage_rewards().set(&max_apr);
        self.min_unbond_epochs().set(&min_unbond_epochs);

        Ok(())
    }

    #[endpoint(stakeFarmThroughProxy)]
    fn stake_farm_through_proxy(
        &self,
        #[payment_multi] farm_tokens: ManagedVec<EsdtTokenPayment<Self::Api>>,
        staked_token_amount: BigUint,
    ) -> SCResult<EnterFarmResultType<Self::Api>> {
        let caller = self.blockchain().get_caller();
        self.require_whitelisted(&caller);

        let staked_token_id = self.farming_token_id().get();
        let staked_token_simulated_payment =
            EsdtTokenPayment::new(staked_token_id, 0, staked_token_amount);

        let mut payments = ManagedVec::from_single_item(staked_token_simulated_payment);
        payments.append_vec(farm_tokens);

        self.stake_farm(payments, OptionalArg::None)
    }

    #[payable("*")]
    #[endpoint(stakeFarm)]
    fn stake_farm(
        &self,
        #[payment_multi] payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<EnterFarmResultType<Self::Api>> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No farm token");

        let payment_0 = payments.get(0).ok_or("empty payments")?;
        let additional_payments = payments.slice(1, payments.len()).unwrap_or_default();

        let token_in = payment_0.token_identifier.clone();
        let enter_amount = payment_0.amount.clone();

        let farming_token_id = self.farming_token_id().get();
        require!(token_in == farming_token_id, "Bad input token");
        require!(enter_amount > 0, "Cannot farm with amount of 0");

        self.generate_aggregated_rewards();

        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let attributes = StakingFarmTokenAttributes {
            reward_per_share: self.reward_per_share().get(),
            entering_epoch: epoch,
            last_claim_block: block,
            initial_farming_amount: enter_amount.clone(),
            compounded_reward: BigUint::zero(),
            current_farm_amount: enter_amount.clone(),
        };

        let caller = self.blockchain().get_caller();
        let farm_token_id = self.farm_token_id().get();
        let (new_farm_token, _created_with_merge) = self.create_farm_tokens_by_merging(
            &enter_amount,
            &farm_token_id,
            &attributes,
            &additional_payments,
        )?;
        self.transfer_execute_custom(
            &caller,
            &farm_token_id,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &opt_accept_funds_func,
        )?;

        Ok(new_farm_token.token_amount)
    }

    #[payable("*")]
    #[endpoint(unstakeFarm)]
    fn unstake_farm(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_nonce] token_nonce: Nonce,
        #[payment_amount] amount: BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<ExitFarmResultType<Self::Api>> {
        self.unstake_farm_common(
            payment_token_id,
            token_nonce,
            amount,
            opt_accept_funds_func,
            false,
        )
    }

    #[payable("*")]
    #[endpoint(unstakeFarmThroughProxy)]
    fn unstake_farm_through_proxy(
        &self,
        #[payment_multi] payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> SCResult<ExitFarmResultType<Self::Api>> {
        let caller = self.blockchain().get_caller();
        self.require_whitelisted(&caller);

        require!(payments.len() == 2, "Invalid payments amount");

        let first_payment = payments.get(0).unwrap();
        let staking_token_id = self.farming_token_id().get();
        require!(
            first_payment.token_identifier == staking_token_id,
            "Invalid first payment"
        );

        let second_payment = payments.get(1).unwrap();
        let farm_token_id = self.farm_token_id().get();
        require!(
            second_payment.token_identifier == farm_token_id,
            "Invalid second payment"
        );

        self.unstake_farm_common(
            second_payment.token_identifier,
            second_payment.token_nonce,
            second_payment.amount,
            OptionalArg::None,
            true,
        )
    }

    fn unstake_farm_common(
        &self,
        payment_token_id: TokenIdentifier,
        token_nonce: Nonce,
        amount: BigUint,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
        is_caller_proxy: bool,
    ) -> SCResult<ExitFarmResultType<Self::Api>> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No farm token");

        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Bad input token");
        require!(amount > 0, "Payment amount cannot be zero");

        let farm_attributes = self.get_attributes::<StakingFarmTokenAttributes<Self::Api>>(
            &payment_token_id,
            token_nonce,
        )?;
        let reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards();

        let mut reward = self.calculate_rewards_with_apr_limit(
            &amount,
            &self.reward_per_share().get(),
            &farm_attributes.reward_per_share,
            farm_attributes.last_claim_block,
        );
        if reward > 0 {
            self.decrease_reward_reserve(&reward)?;
        }

        let initial_farming_token_amount = self.rule_of_three_non_zero_result(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.initial_farming_amount,
        );
        reward += self.rule_of_three(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        );

        let caller = self.blockchain().get_caller();
        self.burn_farm_tokens(&payment_token_id, token_nonce, &amount);

        let farm_token_payment = if !is_caller_proxy {
            let min_unbond_epochs = self.min_unbond_epochs().get();
            let current_epoch = self.blockchain().get_block_epoch();
            let nft_nonce = self.nft_create_tokens(
                &farm_token_id,
                &initial_farming_token_amount,
                &UnbondSftAttributes {
                    unlock_epoch: current_epoch + min_unbond_epochs,
                },
            );
            self.transfer_execute_custom(
                &caller,
                &farm_token_id,
                nft_nonce,
                &initial_farming_token_amount,
                &opt_accept_funds_func,
            )?;

            EsdtTokenPayment::new(farm_token_id, nft_nonce, initial_farming_token_amount)
        } else {
            EsdtTokenPayment::no_payment()
        };

        self.send_rewards(&reward_token_id, &reward, &caller, &opt_accept_funds_func)?;

        Ok(MultiResult2::from((
            farm_token_payment,
            EsdtTokenPayment::new(reward_token_id, 0, reward),
        )))
    }

    #[payable("*")]
    #[endpoint(unbondFarm)]
    fn unbond_farm(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_nonce] token_nonce: Nonce,
        #[payment_amount] amount: BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<UnbondFarmResultType<Self::Api>> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No farm token");

        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Bad input token");
        require!(amount > 0, "Payment amount cannot be zero");

        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            &farm_token_id,
            token_nonce,
        );
        let unlock_epoch = token_info
            .decode_attributes::<UnbondSftAttributes>()?
            .unlock_epoch;
        let current_epoch = self.blockchain().get_block_epoch();
        require!(current_epoch >= unlock_epoch, "Unbond period not over");

        let caller = self.blockchain().get_caller();
        let farming_token_id = self.farming_token_id().get();
        self.transfer_execute_custom(
            &caller,
            &farming_token_id,
            0,
            &amount,
            &opt_accept_funds_func,
        )?;

        Ok(EsdtTokenPayment::new(farming_token_id, 0, amount))
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        #[payment_multi] payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<ClaimRewardsResultType<Self::Api>> {
        self.claim_rewards_common(payments, opt_accept_funds_func, None)
    }

    #[payable("*")]
    #[endpoint(claimRewardsWithNewValue)]
    fn claim_rewards_with_new_value(
        &self,
        #[payment_multi] payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
        new_values: ManagedVec<BigUint>,
    ) -> SCResult<ClaimRewardsResultType<Self::Api>> {
        let caller = self.blockchain().get_caller();
        self.require_whitelisted(&caller);

        require!(
            payments.len() == new_values.len(),
            "Arguments length mismatch"
        );

        self.claim_rewards_common(payments, OptionalArg::None, Some(new_values))
    }

    fn claim_rewards_common(
        &self,
        payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
        opt_new_farm_values: Option<ManagedVec<BigUint>>,
    ) -> SCResult<ClaimRewardsResultType<Self::Api>> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No farm token");

        let payment_0 = payments.get(0).ok_or("empty payments")?;
        let additional_payments = payments.slice(1, payments.len()).unwrap_or_default();

        let payment_token_id = payment_0.token_identifier.clone();
        let old_farming_amount = payment_0.amount.clone();
        let token_nonce = payment_0.token_nonce;

        require!(old_farming_amount > 0, "Zero amount");
        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Unknown farm token");
        let farm_attributes = self.get_attributes::<StakingFarmTokenAttributes<Self::Api>>(
            &payment_token_id,
            token_nonce,
        )?;

        let reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards();

        let reward = self.calculate_rewards_with_apr_limit(
            &old_farming_amount,
            &self.reward_per_share().get(),
            &farm_attributes.reward_per_share,
            farm_attributes.last_claim_block,
        );
        if reward > 0 {
            self.decrease_reward_reserve(&reward)?;
        }

        let new_compound_reward_amount = self.rule_of_three(
            &old_farming_amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        );
        let new_farming_amount = match &opt_new_farm_values {
            Some(new_values) => new_values.get(0).unwrap(),
            None => old_farming_amount.clone(),
        };

        let new_attributes = StakingFarmTokenAttributes {
            reward_per_share: self.reward_per_share().get(),
            entering_epoch: farm_attributes.entering_epoch,
            last_claim_block: self.blockchain().get_block_nonce(),
            initial_farming_amount: new_farming_amount.clone(),
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: new_farming_amount.clone(),
        };

        let caller = self.blockchain().get_caller();
        self.burn_farm_tokens(&payment_token_id, token_nonce, &old_farming_amount);

        let (new_farm_token, _created_with_merge) = match opt_new_farm_values {
            Some(new_farm_values) => {
                let new_additional_values = new_farm_values
                    .slice(1, new_farm_values.len())
                    .unwrap_or_default();

                let mut additional_payments_attributes = ManagedVec::new();
                for (p, new_val) in additional_payments.iter().zip(new_additional_values.iter()) {
                    let mut attr = self.get_attributes::<StakingFarmTokenAttributes<Self::Api>>(
                        &p.token_identifier,
                        p.token_nonce,
                    )?;
                    attr.initial_farming_amount = new_val.clone();
                    attr.current_farm_amount = new_val;

                    additional_payments_attributes.push(attr);
                }

                self.create_farm_tokens_by_merging_with_updated_attributes(
                    &new_farming_amount,
                    &farm_token_id,
                    &new_attributes,
                    &additional_payments,
                    &additional_payments_attributes,
                )?
            }
            None => self.create_farm_tokens_by_merging(
                &new_farming_amount,
                &farm_token_id,
                &new_attributes,
                &additional_payments,
            )?,
        };
        self.transfer_execute_custom(
            &caller,
            &farm_token_id,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &opt_accept_funds_func,
        )?;

        self.send_rewards(&reward_token_id, &reward, &caller, &opt_accept_funds_func)?;

        Ok(MultiResult2::from((
            new_farm_token.token_amount,
            EsdtTokenPayment::new(reward_token_id, 0, reward),
        )))
    }

    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<CompoundRewardsResultType<Self::Api>> {
        require!(self.is_active(), "Not active");

        let payments_vec = self.get_all_payments_managed_vec();
        let payment_0 = payments_vec.get(0).ok_or("empty payments")?;
        let additional_payments = payments_vec
            .slice(1, payments_vec.len())
            .unwrap_or_default();

        let payment_token_id = payment_0.token_identifier.clone();
        let payment_amount = payment_0.amount.clone();
        let payment_token_nonce = payment_0.token_nonce;
        require!(payment_amount > 0, "Zero amount");

        require!(!self.farm_token_id().is_empty(), "No farm token");
        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Unknown farm token");

        let farming_token = self.farming_token_id().get();
        let reward_token = self.reward_token_id().get();
        require!(
            farming_token == reward_token,
            "Farming token differ from reward token"
        );
        self.generate_aggregated_rewards();

        let current_rps = self.reward_per_share().get();
        let farm_attributes = self.get_attributes::<StakingFarmTokenAttributes<Self::Api>>(
            &payment_token_id,
            payment_token_nonce,
        )?;
        let reward = self.calculate_rewards_with_apr_limit(
            &payment_amount,
            &current_rps,
            &farm_attributes.reward_per_share,
            farm_attributes.last_claim_block,
        );

        if reward > 0 {
            self.decrease_reward_reserve(&reward)?;
        }

        let new_farm_contribution = &payment_amount + &reward;
        let new_initial_farming_amount = self.rule_of_three_non_zero_result(
            &payment_amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.initial_farming_amount,
        );
        let new_compound_reward_amount = &self.rule_of_three(
            &payment_amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        ) + &reward;

        let current_epoch = self.blockchain().get_block_epoch();
        let current_block = self.blockchain().get_block_nonce();
        let new_attributes = StakingFarmTokenAttributes {
            reward_per_share: current_rps,
            entering_epoch: current_epoch,
            last_claim_block: current_block,
            initial_farming_amount: new_initial_farming_amount,
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: new_farm_contribution.clone(),
        };

        self.burn_farm_tokens(&farm_token_id, payment_token_nonce, &payment_amount);
        let caller = self.blockchain().get_caller();
        let (new_farm_token, _created_with_merge) = self.create_farm_tokens_by_merging(
            &new_farm_contribution,
            &farm_token_id,
            &new_attributes,
            &additional_payments,
        )?;
        self.transfer_execute_custom(
            &caller,
            &farm_token_id,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &opt_accept_funds_func,
        )?;

        Ok(new_farm_token.token_amount)
    }

    fn burn_farming_tokens(
        &self,
        farming_token_id: &TokenIdentifier,
        farming_amount: &BigUint,
        reward_token_id: &TokenIdentifier,
    ) -> SCResult<()> {
        let pair_contract_address = self.pair_contract_address().get();
        if pair_contract_address.is_zero() {
            self.send()
                .esdt_local_burn(farming_token_id, 0, farming_amount);
        } else {
            let gas_limit = self.burn_gas_limit().get();
            self.pair_contract_proxy(pair_contract_address)
                .remove_liquidity_and_burn_token(
                    farming_token_id.clone(),
                    0,
                    farming_amount.clone(),
                    reward_token_id.clone(),
                )
                .with_gas_limit(gas_limit)
                .transfer_execute();
        }

        Ok(())
    }

    fn create_farm_tokens_by_merging(
        &self,
        amount: &BigUint,
        token_id: &TokenIdentifier,
        attributes: &StakingFarmTokenAttributes<Self::Api>,
        additional_payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> SCResult<(StakingFarmToken<Self::Api>, bool)> {
        let current_position_replic = StakingFarmToken {
            token_amount: self.create_payment(token_id, 0, amount),
            attributes: attributes.clone(),
        };

        let additional_payments_len = additional_payments.len();
        let merged_attributes = self.get_merged_farm_token_attributes(
            additional_payments,
            Some(current_position_replic),
            None,
        )?;
        self.burn_farm_tokens_from_payments(additional_payments);

        let new_amount = &merged_attributes.current_farm_amount;
        let new_nonce = self.mint_farm_tokens(token_id, new_amount, &merged_attributes);

        let new_farm_token = StakingFarmToken {
            token_amount: self.create_payment(token_id, new_nonce, new_amount),
            attributes: merged_attributes,
        };
        let is_merged = additional_payments_len != 0;

        Ok((new_farm_token, is_merged))
    }

    fn create_farm_tokens_by_merging_with_updated_attributes(
        &self,
        amount: &BigUint,
        token_id: &TokenIdentifier,
        attributes: &StakingFarmTokenAttributes<Self::Api>,
        additional_payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        new_additional_attributes: &ManagedVec<StakingFarmTokenAttributes<Self::Api>>,
    ) -> SCResult<(StakingFarmToken<Self::Api>, bool)> {
        let current_position_replic = StakingFarmToken {
            token_amount: self.create_payment(token_id, 0, amount),
            attributes: attributes.clone(),
        };

        let additional_payments_len = additional_payments.len();
        let merged_attributes = self.get_merged_farm_token_attributes(
            additional_payments,
            Some(current_position_replic),
            Some(new_additional_attributes),
        )?;
        self.burn_farm_tokens_from_payments(additional_payments);

        let new_amount = &merged_attributes.current_farm_amount;
        let new_nonce = self.mint_farm_tokens(token_id, new_amount, &merged_attributes);

        let new_farm_token = StakingFarmToken {
            token_amount: self.create_payment(token_id, new_nonce, new_amount),
            attributes: merged_attributes,
        };
        let is_merged = additional_payments_len != 0;

        Ok((new_farm_token, is_merged))
    }

    fn send_back_farming_tokens(
        &self,
        farming_token_id: &TokenIdentifier,
        farming_amount: &BigUint,
        destination: &ManagedAddress,
        opt_accept_funds_func: &OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        self.transfer_execute_custom(
            destination,
            farming_token_id,
            0,
            farming_amount,
            opt_accept_funds_func,
        )?;
        Ok(())
    }

    fn send_rewards(
        &self,
        reward_token_id: &TokenIdentifier,
        reward_amount: &BigUint,
        destination: &ManagedAddress,
        opt_accept_funds_func: &OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        if reward_amount > &mut 0 {
            self.transfer_execute_custom(
                destination,
                reward_token_id,
                0,
                reward_amount,
                opt_accept_funds_func,
            )?;
        }
        Ok(())
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        amount: BigUint,
        attributes: StakingFarmTokenAttributes<Self::Api>,
    ) -> SCResult<BigUint> {
        require!(amount > 0, "Zero liquidity input");
        let farm_token_supply = self.farm_token_supply().get();
        require!(farm_token_supply >= amount, "Not enough supply");

        let last_reward_nonce = self.last_reward_block_nonce().get();
        let current_block_nonce = self.blockchain().get_block_nonce();
        let reward_increase =
            self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);
        let reward_per_share_increase =
            self.calculate_reward_per_share_increase(&reward_increase, &farm_token_supply);

        let future_reward_per_share = self.reward_per_share().get() + reward_per_share_increase;

        Ok(self.calculate_rewards_with_apr_limit(
            &amount,
            &future_reward_per_share,
            &attributes.reward_per_share,
            attributes.last_claim_block,
        ))
    }
}
