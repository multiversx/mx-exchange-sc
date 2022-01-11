#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

pub mod contexts;
pub mod ctx_events;
pub mod custom_config;
pub mod custom_rewards;
pub mod errors;
pub mod farm_token_merge;

use common_structs::{Epoch, FarmTokenAttributes, Nonce};
use config::State;
use errors::*;
use farm_token::FarmToken;

use crate::contexts::base::*;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use config::{
    DEFAULT_BURN_GAS_LIMIT, DEFAULT_MINUMUM_FARMING_EPOCHS, DEFAULT_PENALTY_PERCENT,
    DEFAULT_TRANSFER_EXEC_GAS_LIMIT, MAX_PENALTY_PERCENT,
};

type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[elrond_wasm::contract]
pub trait Farm:
    custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + custom_config::CustomConfigModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
    + farm_token::FarmTokenModule
    + farm_token_merge::FarmTokenMergeModule
    + events::EventsModule
    + contexts::ctx_helper::CtxHelper
    + ctx_events::ContextEventsModule
{
    #[proxy]
    fn locked_asset_factory(&self, to: ManagedAddress) -> factory::Proxy<Self::Api>;

    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[init]
    fn init(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        locked_asset_factory_address: ManagedAddress,
        division_safety_constant: BigUint,
        pair_contract_address: ManagedAddress,
    ) -> SCResult<()> {
        assert!(self, reward_token_id.is_esdt(), ERROR_NOT_AN_ESDT);
        assert!(self, farming_token_id.is_esdt(), ERROR_NOT_AN_ESDT);
        assert!(self, division_safety_constant != 0u64, ERROR_ZERO_AMOUNT);
        let farm_token = self.farm_token_id().get();
        assert!(self, reward_token_id != farm_token, ERROR_SAME_TOKEN_IDS);
        assert!(self, farming_token_id != farm_token, ERROR_SAME_TOKEN_IDS);

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
        self.locked_asset_factory_address()
            .set(&locked_asset_factory_address);
        self.pair_contract_address().set(&pair_contract_address);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> EnterFarmResultType<Self::Api> {
        let mut context = self.new_enter_farm_context(opt_accept_funds_func);

        self.load_state(&mut context);
        assert!(
            self,
            context.get_contract_state() == &State::Active,
            ERROR_NOT_ACTIVE
        );

        self.load_farm_token_id(&mut context);
        assert!(
            self,
            !context.get_farm_token_id().is_empty(),
            ERROR_NO_FARM_TOKEN,
        );

        self.load_farming_token_id(&mut context);
        assert!(self, context.is_accepted_payment(), ERROR_BAD_PAYMENTS,);

        self.load_reward_token_id(&mut context);
        self.load_block_nonce(&mut context);
        self.load_block_epoch(&mut context);
        self.load_reward_per_share(&mut context);
        self.load_farm_token_supply(&mut context);
        self.load_division_safety_constant(&mut context);
        self.generate_aggregated_rewards(&mut context);

        let first_payment_amount = context
            .get_tx_input()
            .get_payments()
            .get_first()
            .amount
            .clone();

        let virtual_position = FarmToken {
            token_amount: self.create_payment(
                context.get_farm_token_id(),
                0,
                &first_payment_amount,
            ),
            attributes: FarmTokenAttributes {
                reward_per_share: context.get_reward_per_share().clone(),
                entering_epoch: context.get_block_epoch(),
                original_entering_epoch: context.get_block_epoch(),
                initial_farming_amount: first_payment_amount.clone(),
                compounded_reward: BigUint::zero(),
                current_farm_amount: first_payment_amount.clone(),
            },
        };

        let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
            &virtual_position,
            context
                .get_tx_input()
                .get_payments()
                .get_additional()
                .unwrap(),
            context.get_storage_cache(),
        );
        context.set_output_position(new_farm_token, created_with_merge);

        self.commit_changes(&context);
        self.execute_output_payments(&context);
        self.emit_enter_farm_event_context(&context);

        context
            .get_output_payments()
            .get(0)
            .as_ref()
            .unwrap()
            .clone()
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(
        &self,
        #[payment_token] _payment_token_id: TokenIdentifier,
        #[payment_nonce] _token_nonce: Nonce,
        #[payment_amount] _amount: BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<ExitFarmResultType<Self::Api>> {
        let mut context = self.new_exit_farm_context(opt_accept_funds_func);

        self.load_state(&mut context);
        assert!(
            self,
            context.get_contract_state() == &State::Active,
            ERROR_NOT_ACTIVE
        );

        self.load_farm_token_id(&mut context);
        assert!(
            self,
            !context.get_farm_token_id().is_empty(),
            ERROR_NO_FARM_TOKEN,
        );

        self.load_farming_token_id(&mut context);
        assert!(self, context.is_accepted_payment(), ERROR_BAD_PAYMENTS,);

        self.load_reward_token_id(&mut context);
        self.load_block_nonce(&mut context);
        self.load_block_epoch(&mut context);
        self.load_reward_per_share(&mut context);
        self.load_farm_token_supply(&mut context);
        self.load_division_safety_constant(&mut context);
        self.generate_aggregated_rewards(&mut context);
        self.load_farm_attributes(&mut context);

        self.generate_aggregated_rewards(&mut context);
        self.calculate_reward(&mut context);
        self.decrease_reward_reserve(&mut context);
        self.calculate_initial_farming_amount(&mut context);
        self.burn_penalty(&mut context);

        self.burn_position(&context);
        self.construct_output_payments(&mut context);
        self.execute_output_payments(context);
        self.emit_exit_farm_event(&context);

        self.construct_and_get_exit_farm_result(&context);
        panic!()

        // let mut reward = self.calculate_reward(
        //     &amount,
        //     &self.reward_per_share().get(),
        //     &farm_attributes.reward_per_share,
        // );
        // if reward > 0 {
        //     self.decrease_reward_reserve(&reward)?;
        // }

        // let mut initial_farming_token_amount = self.rule_of_three_non_zero_result(
        //     &amount,
        //     &farm_attributes.current_farm_amount,
        //     &farm_attributes.initial_farming_amount,
        // )?;
        // reward += self.rule_of_three(
        //     &amount,
        //     &farm_attributes.current_farm_amount,
        //     &farm_attributes.compounded_reward,
        // );

        // if self.should_apply_penalty(farm_attributes.entering_epoch) {
        //     let penalty_amount = self.get_penalty_amount(&initial_farming_token_amount);
        //     if penalty_amount > 0 {
        //         self.burn_farming_tokens(&farming_token_id, &penalty_amount, &reward_token_id)?;
        //         initial_farming_token_amount -= penalty_amount;
        //     }
        // }

        // let caller = self.blockchain().get_caller();
        // self.burn_farm_tokens(&payment_token_id, token_nonce, &amount);
        // self.send_back_farming_tokens(
        //     &farming_token_id,
        //     &initial_farming_token_amount,
        //     &caller,
        //     &opt_accept_funds_func,
        // )?;

        // let mut reward_nonce = 0u64;
        // self.send_rewards(
        //     &mut reward_token_id,
        //     &mut reward_nonce,
        //     &mut reward,
        //     &caller,
        //     farm_attributes.original_entering_epoch,
        //     &opt_accept_funds_func,
        // )?;

        // self.emit_exit_farm_event(
        //     &caller,
        //     &farming_token_id,
        //     &initial_farming_token_amount,
        //     &farm_token_id,
        //     token_nonce,
        //     &amount,
        //     &self.farm_token_supply().get(),
        //     &reward_token_id,
        //     reward_nonce,
        //     &reward,
        //     &self.reward_reserve().get(),
        //     &farm_attributes,
        // );
        // Ok(MultiResult2::from((
        //     self.create_payment(&farming_token_id, 0, &initial_farming_token_amount),
        //     self.create_payment(&reward_token_id, reward_nonce, &reward),
        // )))
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<ClaimRewardsResultType<Self::Api>> {
        assert!(self, self.is_active(), ERROR_NOT_ACTIVE);
        assert!(self, !self.farm_token_id().is_empty(), ERROR_NO_FARM_TOKEN);

        let payments_vec = self.get_all_payments_managed_vec();
        let mut payments_iter = payments_vec.iter();
        let payment_0 = payments_iter.next().ok_or(ERROR_BAD_PAYMENTS_LEN)?;

        let payment_token_id = payment_0.token_identifier.clone();
        let amount = payment_0.amount.clone();
        let token_nonce = payment_0.token_nonce;

        assert!(self, amount > 0, ERROR_ZERO_AMOUNT);
        let farm_token_id = self.farm_token_id().get();
        assert!(
            self,
            payment_token_id == farm_token_id,
            ERROR_BAD_INPUT_TOKEN
        );
        let farm_attributes = self.get_farm_attributes(&payment_token_id, token_nonce)?;

        let mut reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards();

        let mut reward = self.calculate_reward(
            &amount,
            &self.reward_per_share().get(),
            &farm_attributes.reward_per_share,
        );
        if reward > 0 {
            self.decrease_reward_reserve(&reward)?;
        }

        let new_initial_farming_amount = self.rule_of_three_non_zero_result(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.initial_farming_amount,
        )?;
        let new_compound_reward_amount = self.rule_of_three(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        );

        let new_attributes = FarmTokenAttributes {
            reward_per_share: self.reward_per_share().get(),
            entering_epoch: farm_attributes.entering_epoch,
            original_entering_epoch: farm_attributes.original_entering_epoch,
            initial_farming_amount: new_initial_farming_amount,
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: amount.clone(),
        };

        let caller = self.blockchain().get_caller();
        self.burn_farm_tokens(&payment_token_id, token_nonce, &amount);
        let farm_amount = amount.clone();
        let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
            &farm_amount,
            &farm_token_id,
            &new_attributes,
            payments_iter,
        )?;
        self.transfer_execute_custom(
            &caller,
            &farm_token_id,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &opt_accept_funds_func,
        )?;

        let mut reward_nonce = 0u64;
        self.send_rewards(
            &mut reward_token_id,
            &mut reward_nonce,
            &mut reward,
            &caller,
            farm_attributes.original_entering_epoch,
            &opt_accept_funds_func,
        )?;

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
            created_with_merge,
        );
        Ok(MultiResult2::from((
            new_farm_token.token_amount,
            self.create_payment(&reward_token_id, reward_nonce, &reward),
        )))
    }

    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<CompoundRewardsResultType<Self::Api>> {
        assert!(self, self.is_active(), ERROR_NOT_ACTIVE);

        let payments_vec = self.get_all_payments_managed_vec();
        let mut payments_iter = payments_vec.iter();
        let payment_0 = payments_iter.next().ok_or(ERROR_BAD_PAYMENTS_LEN)?;

        let payment_token_id = payment_0.token_identifier.clone();
        let payment_amount = payment_0.amount.clone();
        let payment_token_nonce = payment_0.token_nonce;
        assert!(self, payment_amount > 0, ERROR_ZERO_AMOUNT);

        assert!(self, !self.farm_token_id().is_empty(), ERROR_NO_FARM_TOKEN);
        let farm_token_id = self.farm_token_id().get();
        assert!(
            self,
            payment_token_id == farm_token_id,
            ERROR_BAD_INPUT_TOKEN
        );

        let farming_token = self.farming_token_id().get();
        let reward_token = self.reward_token_id().get();
        assert!(
            self,
            farming_token == reward_token,
            ERROR_DIFFERENT_TOKEN_IDS
        );
        self.generate_aggregated_rewards();

        let current_rps = self.reward_per_share().get();
        let farm_attributes = self.get_farm_attributes(&payment_token_id, payment_token_nonce)?;
        let reward = self.calculate_reward(
            &payment_amount,
            &current_rps,
            &farm_attributes.reward_per_share,
        );

        if reward > 0 {
            self.decrease_reward_reserve(&reward)?;
        }

        let new_farm_contribution = &payment_amount + &reward;
        let new_initial_farming_amount = self.rule_of_three_non_zero_result(
            &payment_amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.initial_farming_amount,
        )?;
        let new_compound_reward_amount = &self.rule_of_three(
            &payment_amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        ) + &reward;

        let compound_original_entering_epoch = self.aggregated_original_entering_epoch_on_compound(
            &payment_token_id,
            &payment_amount,
            &farm_attributes,
            &reward,
        );
        let new_attributes = FarmTokenAttributes {
            reward_per_share: current_rps,
            entering_epoch: self.blockchain().get_block_epoch(),
            original_entering_epoch: compound_original_entering_epoch,
            initial_farming_amount: new_initial_farming_amount,
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: new_farm_contribution.clone(),
        };

        self.burn_farm_tokens(&farm_token_id, payment_token_nonce, &payment_amount);
        let caller = self.blockchain().get_caller();
        let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
            &new_farm_contribution,
            &farm_token_id,
            &new_attributes,
            payments_iter,
        )?;
        self.transfer_execute_custom(
            &caller,
            &farm_token_id,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &opt_accept_funds_func,
        )?;

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
            created_with_merge,
        );
        Ok(new_farm_token.token_amount)
    }

    fn aggregated_original_entering_epoch_on_compound(
        &self,
        farm_token_id: &TokenIdentifier,
        position_amount: &BigUint,
        position_attributes: &FarmTokenAttributes<Self::Api>,
        reward_amount: &BigUint,
    ) -> u64 {
        if reward_amount == &0 {
            return position_attributes.original_entering_epoch;
        }

        let initial_position = FarmToken {
            token_amount: self.create_payment(farm_token_id, 0, position_amount),
            attributes: position_attributes.clone(),
        };

        let mut reward_position = initial_position.clone();
        reward_position.token_amount.amount = reward_amount.clone();
        reward_position.attributes.original_entering_epoch = self.blockchain().get_block_epoch();

        let mut items = ManagedVec::new();
        items.push(initial_position);
        items.push(reward_position);
        self.aggregated_original_entering_epoch(&items)
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
                .execute_on_dest_context_ignore_result();
        }

        Ok(())
    }

    fn create_farm_tokens_by_merging(
        &self,
        virtual_position: &FarmToken<Self::Api>,
        additional_positions: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        storage_cache: &StorageCache<Self::Api>,
    ) -> (FarmToken<Self::Api>, bool) {
        let additional_payments_len = additional_positions.len();
        let merged_attributes =
            self.get_merged_farm_token_attributes(additional_positions, Some(virtual_position));

        self.burn_farm_tokens_from_payments(additional_positions);

        let new_amount = merged_attributes.current_farm_amount.clone();
        let new_nonce = self.mint_farm_tokens(
            &storage_cache.farm_token_id,
            &new_amount,
            &merged_attributes,
        );

        let new_farm_token = FarmToken {
            token_amount: self.create_payment(&storage_cache.farm_token_id, new_nonce, &new_amount),
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
        reward_token_id: &mut TokenIdentifier,
        reward_nonce: &mut Nonce,
        reward_amount: &mut BigUint,
        destination: &ManagedAddress,
        entering_epoch: Epoch,
        opt_accept_funds_func: &OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        if reward_amount > &mut 0 {
            let locked_asset_factory_address = self.locked_asset_factory_address().get();
            let result = self
                .locked_asset_factory(locked_asset_factory_address)
                .create_and_forward(
                    reward_amount.clone(),
                    destination.clone(),
                    entering_epoch,
                    opt_accept_funds_func.clone(),
                )
                .execute_on_dest_context_custom_range(|_, after| (after - 1, after));
            *reward_token_id = result.token_identifier;
            *reward_nonce = result.token_nonce;
            *reward_amount = result.amount;
        }
        Ok(())
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        amount: BigUint,
        attributes: FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        assert!(self, amount > 064, ERROR_ZERO_AMOUNT);
        let farm_token_supply = self.farm_token_supply().get();
        assert!(self, farm_token_supply >= amount, ERROR_NOT_ENOUGH_SUPPLY);

        let last_reward_nonce = self.last_reward_block_nonce().get();
        let current_block_nonce = self.blockchain().get_block_nonce();
        let reward_increase =
            self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);

        let reward_per_share_increase =
            self.calculate_reward_per_share_increase(&reward_increase, &farm_token_supply);
        let future_reward_per_share = self.reward_per_share().get() + reward_per_share_increase;
        let mut reward = self.calculate_reward(
            &amount,
            &future_reward_per_share,
            &attributes.reward_per_share,
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
        amount * self.penalty_percent().get() / MAX_PENALTY_PERCENT
    }
}
