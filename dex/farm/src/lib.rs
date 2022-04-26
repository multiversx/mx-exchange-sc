#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

pub mod custom_rewards;
pub mod farm_token_merge;

use common_errors::*;

use common_structs::{Epoch, FarmTokenAttributes};
use config::State;
use contexts::generic::{GenericContext, StorageCache};
use farm_token::FarmToken;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use config::MAX_PERCENT;

type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[elrond_wasm::contract]
pub trait Farm:
    custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
    + farm_token::FarmTokenModule
    + farm_token_merge::FarmTokenMergeModule
    + events::EventsModule
    + contexts::ctx_helper::CtxHelper
    + migration_from_v1_2::MigrationModule
{
    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[init]
    fn init(&self, reward_reserve: BigUint) {
        self.reward_reserve().update(|x| *x += &reward_reserve);
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm(
        &self,
        #[var_args] opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> EnterFarmResultType<Self::Api> {
        let mut context = self.new_farm_context(opt_accept_funds_func);

        self.load_state(&mut context);
        require!(
            context.get_contract_state().unwrap() == &State::Active,
            ERROR_NOT_ACTIVE
        );

        self.load_farm_token_id(&mut context);
        require!(
            !context.get_farm_token_id().unwrap().is_empty(),
            ERROR_NO_FARM_TOKEN
        );

        self.load_farming_token_id(&mut context);
        require!(context.is_accepted_payment_enter(), ERROR_BAD_PAYMENTS);

        self.load_reward_token_id(&mut context);
        self.load_reward_reserve(&mut context);
        self.load_block_nonce(&mut context);
        self.load_block_epoch(&mut context);
        self.load_reward_per_share(&mut context);
        self.load_farm_token_supply(&mut context);
        self.load_division_safety_constant(&mut context);
        self.generate_aggregated_rewards(context.get_storage_cache_mut());

        let first_payment_amount = context
            .get_tx_input()
            .get_payments()
            .get_first()
            .amount
            .clone();

        let virtual_position_token_amount = self.create_payment(
            context.get_farm_token_id().unwrap(),
            0,
            &first_payment_amount,
        );
        let virtual_position_attributes = FarmTokenAttributes {
            reward_per_share: context.get_reward_per_share().unwrap().clone(),
            entering_epoch: context.get_block_epoch(),
            original_entering_epoch: context.get_block_epoch(),
            initial_farming_amount: first_payment_amount.clone(),
            compounded_reward: BigUint::zero(),
            current_farm_amount: first_payment_amount,
        };
        let virtual_position = FarmToken {
            token_amount: virtual_position_token_amount,
            attributes: virtual_position_attributes,
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
        self.emit_enter_farm_event(&context);

        context.get_output_payments().get(0)
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(
        &self,
        #[var_args] opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> ExitFarmResultType<Self::Api> {
        let mut context = self.new_farm_context(opt_accept_funds_func);

        self.load_state(&mut context);
        require!(
            context.get_contract_state().unwrap() == &State::Active,
            ERROR_NOT_ACTIVE
        );

        self.load_farm_token_id(&mut context);
        require!(
            !context.get_farm_token_id().unwrap().is_empty(),
            ERROR_NO_FARM_TOKEN
        );

        self.load_farming_token_id(&mut context);
        require!(context.is_accepted_payment_exit(), ERROR_BAD_PAYMENTS);

        self.load_reward_token_id(&mut context);
        self.load_reward_reserve(&mut context);
        self.load_block_nonce(&mut context);
        self.load_block_epoch(&mut context);
        self.load_reward_per_share(&mut context);
        self.load_farm_token_supply(&mut context);
        self.load_division_safety_constant(&mut context);
        self.load_farm_attributes(&mut context);

        self.generate_aggregated_rewards(context.get_storage_cache_mut());
        self.calculate_reward(&mut context);
        context.decrease_reward_reserve();
        self.calculate_initial_farming_amount(&mut context);
        self.increase_reward_with_compounded_rewards(&mut context);

        self.commit_changes(&context);
        self.burn_penalty(&mut context);
        self.burn_position(&context);

        self.send_rewards(&mut context);
        self.construct_output_payments_exit(&mut context);
        self.execute_output_payments(&context);
        self.emit_exit_farm_event(&context);

        self.construct_and_get_result(&context)
    }

    #[payable("*")]
    #[endpoint(exitFarmPosition)]
    fn exit_farm_position(
        &self,
        #[var_args] opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> ExitFarmResultType<Self::Api> {
        let mut context = self.new_farm_context(opt_accept_funds_func);

        self.load_state(&mut context);
        require!(
            context.get_contract_state().unwrap() == &State::Active,
            ERROR_NOT_ACTIVE
        );

        self.load_farm_token_id(&mut context);
        require!(
            !context.get_farm_token_id().unwrap().is_empty(),
            ERROR_NO_FARM_TOKEN
        );

        self.load_farming_token_id(&mut context);
        self.load_reward_token_id(&mut context);
        self.load_reward_reserve(&mut context);
        self.load_block_nonce(&mut context);
        self.load_block_epoch(&mut context);
        self.load_reward_per_share(&mut context);
        self.load_farm_token_supply(&mut context);
        self.load_division_safety_constant(&mut context);
        self.load_farm_attributes(&mut context);

        let payments = context.get_tx_input().get_payments();

        let farm_token_payment = payments.get_first();
        let accepted_farm_token = &farm_token_payment.token_identifier
            == context.get_farm_token_id().unwrap()
            && farm_token_payment.token_nonce > 0
            && farm_token_payment.amount > 0;
        require!(accepted_farm_token, ERROR_BAD_PAYMENTS);

        if let Some(additional_payments) = payments.get_additional() {
            require!(additional_payments.len() == 1, ERROR_BAD_PAYMENTS_LEN);
            let additional_payment = additional_payments.get(0);
            let accepted_additional_payment = &additional_payment.token_identifier
                == context.get_reward_token_id().unwrap()
                && additional_payment.token_nonce == 0
                && additional_payment.amount > 0;
            require!(accepted_additional_payment, ERROR_BAD_INPUT_TOKEN);
        }

        self.calculate_initial_farming_amount(&mut context);
        context.set_final_reward(EsdtTokenPayment::new(
            context.get_reward_token_id().unwrap().clone(),
            0,
            BigUint::zero(),
        ));
        self.commit_changes(&context);
        self.burn_penalty(&mut context);

        self.send().esdt_local_burn(
            context.get_farm_token_id().unwrap(),
            context
                .get_tx_input()
                .get_payments()
                .get_first()
                .token_nonce,
            &context.get_tx_input().get_payments().get_first().amount,
        );

        self.construct_output_payments_exit(&mut context);
        self.execute_output_payments(&context);

        self.construct_and_get_result(&context)
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        #[var_args] opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> ClaimRewardsResultType<Self::Api> {
        let mut context = self.new_farm_context(opt_accept_funds_func);

        self.load_state(&mut context);
        require!(
            context.get_contract_state().unwrap() == &State::Active,
            ERROR_NOT_ACTIVE
        );

        self.load_farm_token_id(&mut context);
        require!(
            !context.get_farm_token_id().unwrap().is_empty(),
            ERROR_NO_FARM_TOKEN
        );

        self.load_farming_token_id(&mut context);
        require!(context.is_accepted_payment_claim(), ERROR_BAD_PAYMENTS);

        self.load_reward_token_id(&mut context);
        self.load_reward_reserve(&mut context);
        self.load_block_nonce(&mut context);
        self.load_block_epoch(&mut context);
        self.load_reward_per_share(&mut context);
        self.load_farm_token_supply(&mut context);
        self.load_division_safety_constant(&mut context);
        self.load_farm_attributes(&mut context);

        self.generate_aggregated_rewards(context.get_storage_cache_mut());
        self.calculate_reward(&mut context);
        context.decrease_reward_reserve();

        self.calculate_initial_farming_amount(&mut context);
        let new_compound_reward_amount = self.calculate_new_compound_reward_amount(&context);

        let virtual_position_token_amount = EsdtTokenPayment::new(
            context.get_farm_token_id().unwrap().clone(),
            0,
            context
                .get_tx_input()
                .get_payments()
                .get_first()
                .amount
                .clone(),
        );
        let virtual_position_attributes = FarmTokenAttributes {
            reward_per_share: context.get_reward_per_share().unwrap().clone(),
            entering_epoch: context.get_input_attributes().unwrap().entering_epoch,
            original_entering_epoch: context
                .get_input_attributes()
                .unwrap()
                .original_entering_epoch,
            initial_farming_amount: context.get_initial_farming_amount().unwrap().clone(),
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: context
                .get_tx_input()
                .get_payments()
                .get_first()
                .amount
                .clone(),
        };
        let virtual_position = FarmToken {
            token_amount: virtual_position_token_amount,
            attributes: virtual_position_attributes,
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

        self.burn_position(&context);
        self.commit_changes(&context);

        self.send_rewards(&mut context);
        self.execute_output_payments(&context);
        self.emit_claim_rewards_event(&context);

        self.construct_and_get_result(&context)
    }

    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(
        &self,
        #[var_args] opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> CompoundRewardsResultType<Self::Api> {
        let mut context = self.new_farm_context(opt_accept_funds_func);

        self.load_state(&mut context);
        require!(
            context.get_contract_state().unwrap() == &State::Active,
            ERROR_NOT_ACTIVE
        );

        self.load_farm_token_id(&mut context);
        require!(
            !context.get_farm_token_id().unwrap().is_empty(),
            ERROR_NO_FARM_TOKEN
        );

        self.load_farming_token_id(&mut context);
        self.load_reward_token_id(&mut context);
        require!(context.is_accepted_payment_compound(), ERROR_BAD_PAYMENTS);

        require!(
            context.get_farming_token_id().unwrap() == context.get_reward_token_id().unwrap(),
            ERROR_DIFFERENT_TOKEN_IDS
        );

        self.load_reward_per_share(&mut context);
        self.load_reward_reserve(&mut context);
        self.load_block_nonce(&mut context);
        self.load_block_epoch(&mut context);
        self.load_farm_token_supply(&mut context);
        self.load_division_safety_constant(&mut context);
        self.load_farm_attributes(&mut context);

        self.generate_aggregated_rewards(context.get_storage_cache_mut());
        self.calculate_reward(&mut context);
        context.decrease_reward_reserve();
        self.calculate_initial_farming_amount(&mut context);

        let virtual_position_amount = &context.get_tx_input().get_payments().get_first().amount
            + context.get_position_reward().unwrap();
        let virtual_position_token_amount = EsdtTokenPayment::new(
            context.get_farm_token_id().unwrap().clone(),
            0,
            virtual_position_amount,
        );

        let virtual_position_compounded_reward = self
            .calculate_new_compound_reward_amount(&context)
            + context.get_position_reward().unwrap();
        let virtual_position_current_farm_amount =
            &context.get_tx_input().get_payments().get_first().amount
                + context.get_position_reward().unwrap();
        let virtual_position_attributes = FarmTokenAttributes {
            reward_per_share: context.get_reward_per_share().unwrap().clone(),
            entering_epoch: context.get_block_epoch(),
            original_entering_epoch: context.get_block_epoch(),
            initial_farming_amount: context.get_initial_farming_amount().unwrap().clone(),
            compounded_reward: virtual_position_compounded_reward,
            current_farm_amount: virtual_position_current_farm_amount,
        };

        let virtual_position = FarmToken {
            token_amount: virtual_position_token_amount,
            attributes: virtual_position_attributes,
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

        self.burn_position(&context);
        self.commit_changes(&context);

        self.execute_output_payments(&context);

        context.set_final_reward_for_emit_compound_event();
        self.emit_compound_rewards_event(&context);

        context.get_output_payments().get(0)
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
            &storage_cache.farm_token_id.clone().unwrap(),
            &new_amount,
            &merged_attributes,
        );

        let new_farm_token = FarmToken {
            token_amount: self.create_payment(
                &storage_cache.farm_token_id.clone().unwrap(),
                new_nonce,
                &new_amount,
            ),
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
        opt_accept_funds_func: &OptionalValue<ManagedBuffer>,
    ) {
        self.transfer_execute_custom(
            destination,
            farming_token_id,
            0,
            farming_amount,
            opt_accept_funds_func,
        );
    }

    fn send_rewards(&self, context: &mut GenericContext<Self::Api>) {
        if context.get_position_reward().unwrap() > &0u64 {
            self.transfer_execute_custom(
                context.get_caller(),
                context.get_reward_token_id().unwrap(),
                0,
                context.get_position_reward().unwrap(),
                context.get_opt_accept_funds_func(),
            );
        }

        context.set_final_reward(self.create_payment(
            context.get_reward_token_id().unwrap(),
            0,
            context.get_position_reward().unwrap(),
        ));
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        amount: BigUint,
        attributes: FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        require!(amount > 0u64, ERROR_ZERO_AMOUNT);
        let farm_token_supply = self.farm_token_supply().get();
        require!(farm_token_supply >= amount, ERROR_ZERO_AMOUNT);

        let last_reward_nonce = self.last_reward_block_nonce().get();
        let current_block_nonce = self.blockchain().get_block_nonce();
        let reward_increase =
            self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);
        let reward_per_share_increase = reward_increase * &self.division_safety_constant().get()
            / self.farm_token_supply().get();

        let future_reward_per_share = self.reward_per_share().get() + reward_per_share_increase;

        if future_reward_per_share > attributes.reward_per_share {
            let reward_per_share_diff = future_reward_per_share - attributes.reward_per_share;
            amount * &reward_per_share_diff / self.division_safety_constant().get()
        } else {
            BigUint::zero()
        }
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

    fn burn_penalty(&self, context: &mut GenericContext<Self::Api>) {
        if self.should_apply_penalty(context.get_input_attributes().unwrap().entering_epoch) {
            let penalty_amount =
                self.get_penalty_amount(context.get_initial_farming_amount().unwrap());
            if penalty_amount > 0u64 {
                self.burn_farming_tokens(
                    context.get_farming_token_id().unwrap(),
                    &penalty_amount,
                    context.get_reward_token_id().unwrap(),
                );
                context.decrease_farming_token_amount(&penalty_amount);
            }
        }
    }

    fn burn_position(&self, context: &GenericContext<Self::Api>) {
        let farm_token = context.get_tx_input().get_payments().get_first();
        self.burn_farm_tokens(
            &farm_token.token_identifier,
            farm_token.token_nonce,
            &farm_token.amount,
        );
    }

    fn calculate_new_compound_reward_amount(&self, context: &GenericContext<Self::Api>) -> BigUint {
        self.rule_of_three(
            &context.get_tx_input().get_payments().get_first().amount,
            &context.get_input_attributes().unwrap().current_farm_amount,
            &context.get_input_attributes().unwrap().compounded_reward,
        )
    }
}
