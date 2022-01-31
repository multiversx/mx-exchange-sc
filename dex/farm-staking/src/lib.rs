#![no_std]
#![feature(generic_associated_types)]
#![feature(exact_size_is_empty)]
#![allow(clippy::too_many_arguments)]

pub mod custom_rewards;
pub mod farm_token_merge;

use common_structs::{Epoch, Nonce};
use config::State;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::farm_token_merge::StakingFarmTokenAttributes;
use config::{
    DEFAULT_BURN_GAS_LIMIT, DEFAULT_MINUMUM_FARMING_EPOCHS, DEFAULT_PENALTY_PERCENT,
    DEFAULT_TRANSFER_EXEC_GAS_LIMIT, MAX_PERCENT,
};
use farm_token_merge::StakingFarmToken;

type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type UnbondFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
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
    ) {
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
    }

    #[payable("*")]
    #[endpoint(stakeFarm)]
    fn stake_farm(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> EnterFarmResultType<Self::Api> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No farm token");

        let payments_vec = self.call_value().all_esdt_transfers();
        let payment_0 = payments_vec
            .try_get(0)
            .unwrap_or_else(|| sc_panic!("empty payments"));
        let additional_payments = payments_vec
            .slice(1, payments_vec.len())
            .unwrap_or_default();

        let token_in = payment_0.token_identifier.clone();
        let enter_amount = payment_0.amount;

        let farming_token_id = self.farming_token_id().get();
        require!(token_in == farming_token_id, "Bad input token");
        require!(enter_amount > 0u32, "Cannot farm with amount of 0");

        self.farming_token_total_liquidity()
            .update(|liq| *liq += &enter_amount);

        let farm_contribution = &enter_amount;
        // let reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards();

        let epoch = self.blockchain().get_block_epoch();
        let block = self.blockchain().get_block_nonce();
        let attributes = StakingFarmTokenAttributes {
            reward_per_share: self.reward_per_share().get(),
            entering_epoch: epoch,
            last_claim_block: block,
            initial_farming_amount: enter_amount.clone(),
            compounded_reward: BigUint::zero(),
            current_farm_amount: farm_contribution.clone(),
        };

        let caller = self.blockchain().get_caller();
        let farm_token_id = self.farm_token_id().get();
        let (new_farm_token, _created_with_merge) = self.create_farm_tokens_by_merging(
            farm_contribution,
            &farm_token_id,
            &attributes,
            &additional_payments,
        );
        self.transfer_execute_custom(
            &caller,
            &farm_token_id,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &opt_accept_funds_func,
        );

        /*
        self.emit_enter_farm_event(
            &caller,
            &farming_token_id,
            &enter_amount,
            &new_farm_token.token_amount.token_identifier,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &self.farm_token_supply().get(),
            &reward_token_id,
            &self.reward_reserve().get(),
            &new_farm_token.attributes,
            created_with_merge
            );
        */
        new_farm_token.token_amount
    }

    #[payable("*")]
    #[endpoint(unstakeFarm)]
    fn unstake_farm(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_nonce] token_nonce: Nonce,
        #[payment_amount] amount: BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> ExitFarmResultType<Self::Api> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No farm token");

        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Bad input token");
        require!(amount > 0, "Payment amount cannot be zero");

        let farm_attributes = self.get_attributes::<StakingFarmTokenAttributes<Self::Api>>(
            &payment_token_id,
            token_nonce,
        );
        let reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards();

        let mut reward = self.calculate_rewards_with_apr_limit(
            &amount,
            &self.reward_per_share().get(),
            &farm_attributes.reward_per_share,
            farm_attributes.last_claim_block,
        );
        if reward > 0 {
            self.decrease_reward_reserve(&reward);
        }

        let farming_token_id = self.farming_token_id().get();
        let mut initial_farming_token_amount = self.rule_of_three_non_zero_result(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.initial_farming_amount,
        );
        reward += self.rule_of_three(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        );

        self.farming_token_total_liquidity()
            .update(|liq| *liq -= &initial_farming_token_amount);

        if self.should_apply_penalty(farm_attributes.entering_epoch) {
            let penalty_amount = self.get_penalty_amount(&initial_farming_token_amount);
            if penalty_amount > 0 {
                self.burn_farming_tokens(&farming_token_id, &penalty_amount, &reward_token_id);
                initial_farming_token_amount -= penalty_amount;
            }
        }

        let caller = self.blockchain().get_caller();
        self.burn_farm_tokens(&payment_token_id, token_nonce, &amount);

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
            &amount,
            &opt_accept_funds_func,
        );

        let reward_nonce = 0u64;
        self.send_rewards(&reward_token_id, &reward, &caller, &opt_accept_funds_func);

        /*
        self.emit_exit_farm_event(
            &caller,
            &farming_token_id,
            &initial_farming_token_amount,
            &farm_token_id,
            token_nonce,
            &amount,
            &self.farm_token_supply().get(),
            &reward_token_id,
            reward_nonce,
            &reward,
            &self.reward_reserve().get(),
            &farm_attributes
            );
        */
        MultiResult2::from((
            self.create_payment(&farm_token_id, nft_nonce, &initial_farming_token_amount),
            self.create_payment(&reward_token_id, reward_nonce, &reward),
        ))
    }

    #[payable("*")]
    #[endpoint(unbondFarm)]
    fn unbond_farm(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_nonce] token_nonce: Nonce,
        #[payment_amount] amount: BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> UnbondFarmResultType<Self::Api> {
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
            .decode_attributes_or_exit::<UnbondSftAttributes>()
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
        );

        EsdtTokenPayment::new(farming_token_id, 0, amount)
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> ClaimRewardsResultType<Self::Api> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No farm token");

        let payments_vec = self.call_value().all_esdt_transfers();
        let payment_0 = payments_vec
            .try_get(0)
            .unwrap_or_else(|| sc_panic!("empty payments"));
        let additional_payments = payments_vec
            .slice(1, payments_vec.len())
            .unwrap_or_default();

        let payment_token_id = payment_0.token_identifier.clone();
        let amount = payment_0.amount.clone();
        let token_nonce = payment_0.token_nonce;

        require!(amount > 0u32, "Zero amount");
        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Unknown farm token");
        let farm_attributes = self.get_attributes::<StakingFarmTokenAttributes<Self::Api>>(
            &payment_token_id,
            token_nonce,
        );

        let reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards();

        let reward = self.calculate_rewards_with_apr_limit(
            &amount,
            &self.reward_per_share().get(),
            &farm_attributes.reward_per_share,
            farm_attributes.last_claim_block,
        );
        if reward > 0u32 {
            self.decrease_reward_reserve(&reward);
        }

        let new_initial_farming_amount = self.rule_of_three_non_zero_result(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.initial_farming_amount,
        );
        let new_compound_reward_amount = self.rule_of_three(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        );

        let new_attributes = StakingFarmTokenAttributes {
            reward_per_share: self.reward_per_share().get(),
            entering_epoch: farm_attributes.entering_epoch,
            last_claim_block: self.blockchain().get_block_nonce(),
            initial_farming_amount: new_initial_farming_amount,
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: amount.clone(),
        };

        let caller = self.blockchain().get_caller();
        self.burn_farm_tokens(&payment_token_id, token_nonce, &amount);
        let farm_amount = amount;
        let (new_farm_token, _created_with_merge) = self.create_farm_tokens_by_merging(
            &farm_amount,
            &farm_token_id,
            &new_attributes,
            &additional_payments,
        );
        self.transfer_execute_custom(
            &caller,
            &farm_token_id,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &opt_accept_funds_func,
        );

        let reward_nonce = 0u64;
        self.send_rewards(&reward_token_id, &reward, &caller, &opt_accept_funds_func);

        /*
        self.emit_claim_rewards_event(
            &caller,
            &farm_token_id,
            token_nonce,
            &amount,
            &new_farm_token.token_amount.token_identifier,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &self.farm_token_supply().get(),
            &reward_token_id,
            reward_nonce,
            &reward,
            &self.reward_reserve().get(),
            &farm_attributes,
            &new_farm_token.attributes,
            created_with_merge
            );
        */
        MultiResult2::from((
            new_farm_token.token_amount,
            self.create_payment(&reward_token_id, reward_nonce, &reward),
        ))
    }

    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> CompoundRewardsResultType<Self::Api> {
        require!(self.is_active(), "Not active");

        let payments_vec = self.call_value().all_esdt_transfers();
        let payment_0 = payments_vec
            .try_get(0)
            .unwrap_or_else(|| sc_panic!("empty payments"));
        let additional_payments = payments_vec
            .slice(1, payments_vec.len())
            .unwrap_or_default();

        let payment_token_id = payment_0.token_identifier.clone();
        let payment_amount = payment_0.amount.clone();
        let payment_token_nonce = payment_0.token_nonce;
        require!(payment_amount > 0u32, "Zero amount");

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
        );
        let reward = self.calculate_rewards_with_apr_limit(
            &payment_amount,
            &current_rps,
            &farm_attributes.reward_per_share,
            farm_attributes.last_claim_block,
        );

        if reward > 0u32 {
            self.decrease_reward_reserve(&reward);
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
        );
        self.transfer_execute_custom(
            &caller,
            &farm_token_id,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &opt_accept_funds_func,
        );

        /*
        self.emit_compound_rewards_event(
            &caller,
            &farm_token_id,
            payment_token_nonce,
            &payment_amount,
            &new_farm_token.token_amount.token_identifier,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &self.farm_token_supply().get(),
            &self.reward_token_id().get(),
            0,
            &reward,
            &self.reward_reserve().get(),
            &farm_attributes,
            &new_farm_token.attributes,
            created_with_merge
            );
        */
        new_farm_token.token_amount
    }

    fn burn_farming_tokens(
        &self,
        farming_token_id: &TokenIdentifier,
        farming_amount: &BigUint,
        reward_token_id: &TokenIdentifier,
    ) {
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
    }

    fn create_farm_tokens_by_merging(
        &self,
        amount: &BigUint,
        token_id: &TokenIdentifier,
        attributes: &StakingFarmTokenAttributes<Self::Api>,
        additional_payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> (StakingFarmToken<Self::Api>, bool) {
        let current_position_replic = StakingFarmToken {
            token_amount: self.create_payment(token_id, 0, amount),
            attributes: attributes.clone(),
        };

        let additional_payments_len = additional_payments.len();
        let merged_attributes = self.get_merged_farm_token_attributes(
            additional_payments.iter(),
            Some(current_position_replic),
        );
        self.burn_farm_tokens_from_payments(additional_payments);

        let new_amount = &merged_attributes.current_farm_amount;
        let new_nonce = self.mint_farm_tokens(token_id, new_amount, &merged_attributes);

        let new_farm_token = StakingFarmToken {
            token_amount: self.create_payment(token_id, new_nonce, new_amount),
            attributes: merged_attributes,
        };
        let is_merged = additional_payments_len != 0;

        (new_farm_token, is_merged)
    }

    fn send_back_farming_tokens(
        &self,
        farming_token_id: &TokenIdentifier,
        farming_amount: &BigUint,
        destination: &ManagedAddress,
        opt_accept_funds_func: &OptionalArg<ManagedBuffer>,
    ) {
        self.transfer_execute_custom(
            destination,
            farming_token_id,
            0,
            farming_amount,
            opt_accept_funds_func,
        );
    }

    fn send_rewards(
        &self,
        reward_token_id: &TokenIdentifier,
        reward_amount: &BigUint,
        destination: &ManagedAddress,
        opt_accept_funds_func: &OptionalArg<ManagedBuffer>,
    ) {
        if reward_amount > &mut 0 {
            self.transfer_execute_custom(
                destination,
                reward_token_id,
                0,
                reward_amount,
                opt_accept_funds_func,
            );
        }
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        amount: BigUint,
        attributes: StakingFarmTokenAttributes<Self::Api>,
    ) -> BigUint {
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
        let mut reward = self.calculate_rewards_with_apr_limit(
            &amount,
            &future_reward_per_share,
            &attributes.reward_per_share,
            attributes.last_claim_block,
        );
        if self.should_apply_penalty(attributes.entering_epoch) {
            let penalty = self.get_penalty_amount(&reward);
            reward -= penalty;
        }

        reward
    }

    #[inline]
    fn should_apply_penalty(&self, entering_epoch: Epoch) -> bool {
        entering_epoch + self.minimum_farming_epochs().get() as u64
            > self.blockchain().get_block_epoch()
    }

    #[inline]
    fn get_penalty_amount(&self, amount: &BigUint) -> BigUint {
        amount * self.penalty_percent().get() / MAX_PERCENT
    }
}
