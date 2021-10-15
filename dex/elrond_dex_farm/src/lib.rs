#![no_std]
#![allow(clippy::too_many_arguments)]

pub mod config;
mod events;
mod farm_token;
pub mod farm_token_merge;
mod rewards;

use common_structs::{
    Epoch, FarmTokenAttributes, FftTokenAmountPair, GenericTokenAmountPair, Nonce,
};
use config::State;
use farm_token::FarmToken;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::config::{
    DEFAULT_LOCKED_REWARDS_LIQUIDITY_MUTIPLIER, DEFAULT_MINUMUM_FARMING_EPOCHS,
    DEFAULT_PENALTY_PERCENT, DEFAULT_TRANSFER_EXEC_GAS_LIMIT, MAX_PENALTY_PERCENT,
};

type EnterFarmResultType<BigUint> = GenericTokenAmountPair<BigUint>;
type CompoundRewardsResultType<BigUint> = GenericTokenAmountPair<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiResult2<GenericTokenAmountPair<BigUint>, GenericTokenAmountPair<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiResult2<FftTokenAmountPair<BigUint>, GenericTokenAmountPair<BigUint>>;

#[elrond_wasm::contract]
pub trait Farm:
    rewards::RewardsModule
    + config::ConfigModule
    + token_supply::TokenSupplyModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
    + farm_token::FarmTokenModule
    + farm_token_merge::FarmTokenMergeModule
    + events::EventsModule
{
    #[proxy]
    fn locked_asset_factory(&self, to: ManagedAddress)
        -> sc_locked_asset_factory::Proxy<Self::Api>;

    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> elrond_dex_pair::Proxy<Self::Api>;

    #[init]
    fn init(
        &self,
        router_address: ManagedAddress,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        locked_asset_factory_address: ManagedAddress,
        division_safety_constant: BigUint,
        pair_contract_address: ManagedAddress,
    ) -> SCResult<()> {
        require!(
            reward_token_id.is_valid_esdt_identifier(),
            "Reward token ID is not a valid esdt identifier"
        );
        require!(
            farming_token_id.is_valid_esdt_identifier(),
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

        self.state().set_if_empty(&State::Active);
        self.penalty_percent()
            .set_if_empty(&DEFAULT_PENALTY_PERCENT);
        self.locked_rewards_apr_multiplier()
            .set_if_empty(&DEFAULT_LOCKED_REWARDS_LIQUIDITY_MUTIPLIER);
        self.minimum_farming_epochs()
            .set_if_empty(&DEFAULT_MINUMUM_FARMING_EPOCHS);
        self.transfer_exec_gas_limit()
            .set_if_empty(&DEFAULT_TRANSFER_EXEC_GAS_LIMIT);
        self.division_safety_constant()
            .set_if_empty(&division_safety_constant);

        self.owner().set(&self.blockchain().get_caller());
        self.router_address().set(&router_address);
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
    ) -> SCResult<EnterFarmResultType<Self::Api>> {
        self.enter_farm_common(false, opt_accept_funds_func)
    }

    #[payable("*")]
    #[endpoint(enterFarmAndLockRewards)]
    fn enter_farm_and_lock_rewards(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<EnterFarmResultType<Self::Api>> {
        self.enter_farm_common(true, opt_accept_funds_func)
    }

    fn enter_farm_common(
        &self,
        with_locked_rewards: bool,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<EnterFarmResultType<Self::Api>> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No issued farm token");

        let payments = self
            .raw_vm_api()
            .get_all_esdt_transfers()
            .into_iter()
            .collect::<Vec<EsdtTokenPayment<Self::Api>>>();
        require!(payments.len() >= 1, "empty payments");

        let token_in = payments[0].token_identifier.clone();
        let enter_amount = payments[0].amount.clone();

        let farming_token_id = self.farming_token_id().get();
        require!(token_in == farming_token_id, "Bad input token");
        require!(enter_amount > 0, "Cannot farm with amount of 0");
        self.increase_farming_token_reserve(&enter_amount);

        let (farm_contribution, apr_multiplier) =
            self.get_farm_contribution(&enter_amount, with_locked_rewards);

        let reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards(&reward_token_id);

        let epoch = self.blockchain().get_block_epoch();
        let attributes = FarmTokenAttributes {
            reward_per_share: self.reward_per_share().get(),
            entering_epoch: epoch,
            original_entering_epoch: epoch,
            apr_multiplier,
            with_locked_rewards,
            initial_farming_amount: enter_amount.clone(),
            compounded_reward: self.types().big_uint_zero(),
            current_farm_amount: farm_contribution.clone(),
        };

        let caller = self.blockchain().get_caller();
        let farm_token_id = self.farm_token_id().get();
        let (new_farm_token, created_with_merge) =
            self.create_farm_tokens_by_merging(&farm_contribution, &farm_token_id, &attributes)?;
        self.send_nft_tokens(
            &farm_token_id,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &caller,
            &opt_accept_funds_func,
        )?;

        let farming_token_amount = FftTokenAmountPair {
            token_id: farming_token_id,
            amount: enter_amount,
        };
        let reward_token_reserve = FftTokenAmountPair {
            token_id: reward_token_id,
            amount: self.reward_reserve().get(),
        };
        self.emit_enter_farm_event(
            caller,
            farming_token_amount,
            self.farming_token_reserve().get(),
            new_farm_token.token_amount.clone(),
            self.get_farm_token_supply(),
            reward_token_reserve,
            new_farm_token.attributes,
            created_with_merge,
        );
        Ok(new_farm_token.token_amount)
    }

    fn get_farm_contribution(&self, amount: &BigUint, with_locked_rewards: bool) -> (BigUint, u8) {
        if with_locked_rewards {
            let multiplier = self.locked_rewards_apr_multiplier().get();
            (amount * (multiplier as u64), multiplier)
        } else {
            (amount.clone(), 1u8)
        }
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<ExitFarmResultType<Self::Api>> {
        require!(!self.farm_token_id().is_empty(), "No issued farm token");

        let payments = self
            .raw_vm_api()
            .get_all_esdt_transfers()
            .into_iter()
            .collect::<Vec<EsdtTokenPayment<Self::Api>>>();
        require!(payments.len() == 1, "bad payment len");

        let payment_token_id = payments[0].token_identifier.clone();
        let amount = payments[0].amount.clone();
        let token_nonce = payments[0].token_nonce;

        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Bad input token");
        require!(amount > 0, "Payment amount cannot be zero");

        let farm_attributes = self.get_farm_attributes(&payment_token_id, token_nonce)?;
        let mut reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards(&reward_token_id);

        let mut reward = self.calculate_reward(
            &amount,
            &self.reward_per_share().get(),
            &farm_attributes.reward_per_share,
        );
        if reward > 0 {
            self.decrease_reward_reserve(&reward)?;
        }

        let farming_token_id = self.farming_token_id().get();
        let mut initial_farming_token_amount = self.rule_of_three_non_zero_result(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.initial_farming_amount,
        )?;
        reward += self.rule_of_three(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        );

        if self.should_apply_penalty(farm_attributes.entering_epoch) {
            let penalty_amount = self.get_penalty_amount(&initial_farming_token_amount);
            if penalty_amount > 0 {
                self.burn_farming_tokens(&farming_token_id, &penalty_amount, &reward_token_id)?;
                initial_farming_token_amount -= penalty_amount;
            }
        }

        let caller = self.blockchain().get_caller();
        self.burn_farm_tokens(&payment_token_id, token_nonce, &amount)?;
        self.send_back_farming_tokens(
            &farming_token_id,
            &initial_farming_token_amount,
            &caller,
            &opt_accept_funds_func,
        )?;

        let mut reward_nonce = 0u64;
        self.send_rewards(
            &mut reward_token_id,
            &mut reward_nonce,
            &mut reward,
            &caller,
            farm_attributes.with_locked_rewards,
            farm_attributes.original_entering_epoch,
            &opt_accept_funds_func,
        )?;

        let farming_token_amount = FftTokenAmountPair {
            token_id: farming_token_id,
            amount: initial_farming_token_amount,
        };
        let reward_token_amount = GenericTokenAmountPair {
            token_id: reward_token_id,
            token_nonce: reward_nonce,
            amount: reward,
        };
        let farm_token_amount = GenericTokenAmountPair {
            token_id: farm_token_id,
            token_nonce,
            amount,
        };
        self.emit_exit_farm_event(
            caller,
            farming_token_amount.clone(),
            self.farming_token_reserve().get(),
            farm_token_amount,
            self.get_farm_token_supply(),
            reward_token_amount.clone(),
            self.reward_reserve().get(),
            farm_attributes,
        );
        Ok((farming_token_amount, reward_token_amount).into())
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<ClaimRewardsResultType<Self::Api>> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No issued farm token");

        let payments = self
            .raw_vm_api()
            .get_all_esdt_transfers()
            .into_iter()
            .collect::<Vec<EsdtTokenPayment<Self::Api>>>();
        require!(payments.len() >= 1, "bad payment len");

        let payment_token_id = payments[0].token_identifier.clone();
        let amount = payments[0].amount.clone();
        let token_nonce = payments[0].token_nonce;

        require!(amount > 0, "Zero amount");
        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Unknown farm token");
        let farm_attributes = self.get_farm_attributes(&payment_token_id, token_nonce)?;

        let mut reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards(&reward_token_id);

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
            apr_multiplier: farm_attributes.apr_multiplier,
            with_locked_rewards: farm_attributes.with_locked_rewards,
            initial_farming_amount: new_initial_farming_amount,
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: amount.clone(),
        };

        let caller = self.blockchain().get_caller();
        self.burn_farm_tokens(&payment_token_id, token_nonce, &amount)?;
        let farm_amount = amount.clone();
        let (new_farm_token, created_with_merge) =
            self.create_farm_tokens_by_merging(&farm_amount, &farm_token_id, &new_attributes)?;
        self.send_nft_tokens(
            &farm_token_id,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &caller,
            &opt_accept_funds_func,
        )?;

        // Send rewards
        let mut reward_nonce = 0u64;
        self.send_rewards(
            &mut reward_token_id,
            &mut reward_nonce,
            &mut reward,
            &caller,
            farm_attributes.with_locked_rewards,
            farm_attributes.original_entering_epoch,
            &opt_accept_funds_func,
        )?;

        let old_farm_token_amount = GenericTokenAmountPair {
            token_id: farm_token_id,
            token_nonce,
            amount,
        };
        let reward_token_amount = GenericTokenAmountPair {
            token_id: reward_token_id,
            token_nonce: reward_nonce,
            amount: reward,
        };

        self.emit_claim_rewards_event(
            caller,
            old_farm_token_amount,
            new_farm_token.token_amount.clone(),
            self.get_farm_token_supply(),
            reward_token_amount.clone(),
            self.reward_reserve().get(),
            farm_attributes,
            new_farm_token.attributes,
            created_with_merge,
        );
        Ok((new_farm_token.token_amount, reward_token_amount).into())
    }

    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<CompoundRewardsResultType<Self::Api>> {
        require!(self.is_active(), "Not active");

        let payments = self
            .raw_vm_api()
            .get_all_esdt_transfers()
            .into_iter()
            .collect::<Vec<EsdtTokenPayment<Self::Api>>>();
        require!(payments.len() >= 1, "bad payment len");

        let payment_token_id = payments[0].token_identifier.clone();
        let payment_amount = payments[0].amount.clone();
        let payment_token_nonce = payments[0].token_nonce;
        require!(payment_amount > 0, "Zero amount");

        require!(!self.farm_token_id().is_empty(), "No issued farm token");
        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Unknown farm token");

        let farming_token = self.farming_token_id().get();
        let reward_token = self.reward_token_id().get();
        require!(
            farming_token == reward_token,
            "Farming token differ from reward token"
        );
        self.generate_aggregated_rewards(&reward_token);

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

        let farm_token_id = self.farm_token_id().get();
        let new_farm_contribution =
            &payment_amount + &(&reward * (farm_attributes.apr_multiplier as u64));

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
            apr_multiplier: farm_attributes.apr_multiplier,
            with_locked_rewards: farm_attributes.with_locked_rewards,
            initial_farming_amount: new_initial_farming_amount,
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: new_farm_contribution.clone(),
        };

        self.burn_farm_tokens(&farm_token_id, payment_token_nonce, &payment_amount)?;
        let caller = self.blockchain().get_caller();
        let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
            &new_farm_contribution,
            &farm_token_id,
            &new_attributes,
        )?;
        self.send_nft_tokens(
            &farm_token_id,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &caller,
            &opt_accept_funds_func,
        )?;

        let old_farm_token_amount = GenericTokenAmountPair {
            token_id: farm_token_id,
            token_nonce: payment_token_nonce,
            amount: payment_amount,
        };
        let reward_token_amount = GenericTokenAmountPair {
            token_id: self.reward_token_id().get(),
            token_nonce: 0,
            amount: reward,
        };

        self.emit_compound_rewards_event(
            caller,
            old_farm_token_amount,
            new_farm_token.token_amount.clone(),
            self.get_farm_token_supply(),
            reward_token_amount,
            self.reward_reserve().get(),
            farm_attributes,
            new_farm_token.attributes,
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
            token_amount: GenericTokenAmountPair {
                token_id: farm_token_id.clone(),
                token_nonce: 0,
                amount: position_amount.clone(),
            },
            attributes: position_attributes.clone(),
        };

        let mut reward_position = initial_position.clone();
        reward_position.token_amount.amount = reward_amount.clone();
        reward_position.attributes.original_entering_epoch = self.blockchain().get_block_epoch();

        self.aggregated_original_entering_epoch(&[initial_position, reward_position])
    }

    fn burn_farming_tokens(
        &self,
        farming_token_id: &TokenIdentifier,
        farming_amount: &BigUint,
        reward_token_id: &TokenIdentifier,
    ) -> SCResult<()> {
        self.decrease_farming_token_reserve(farming_amount)?;

        let zero_address = self.types().managed_address_zero();
        let pair_contract_address = self.pair_contract_address().get();

        if pair_contract_address == zero_address {
            self.burn_tokens(farming_token_id, farming_amount);
        } else {
            self.pair_contract_proxy(pair_contract_address)
                .remove_liquidity_and_burn_token(
                    farming_token_id.clone(),
                    farming_amount.clone(),
                    reward_token_id.clone(),
                )
                .execute_on_dest_context_ignore_result();
        }

        Ok(())
    }

    fn create_farm_tokens_by_merging(
        &self,
        amount: &BigUint,
        token_id: &TokenIdentifier,
        attributes: &FarmTokenAttributes<Self::Api>,
    ) -> SCResult<(FarmToken<Self::Api>, bool)> {
        let current_position_replic = FarmToken {
            token_amount: GenericTokenAmountPair {
                token_id: token_id.clone(),
                token_nonce: 0,
                amount: amount.clone(),
            },
            attributes: attributes.clone(),
        };

        let mut additional_payments = self
            .raw_vm_api()
            .get_all_esdt_transfers()
            .into_iter()
            .collect::<Vec<EsdtTokenPayment<Self::Api>>>();
        additional_payments.remove(0);

        let additional_payments_len = additional_payments.len();
        let merged_attributes = self.get_merged_farm_token_attributes(
            &additional_payments,
            Some(current_position_replic),
        )?;
        self.burn_farm_tokens_from_payments(&additional_payments)?;

        let new_amount = merged_attributes.current_farm_amount.clone();
        let new_attributes = merged_attributes;
        let new_nonce = self.create_farm_tokens(&new_amount, token_id, &new_attributes);

        let new_farm_token = FarmToken {
            token_amount: GenericTokenAmountPair {
                token_id: token_id.clone(),
                token_nonce: new_nonce,
                amount: new_amount,
            },
            attributes: new_attributes,
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
        self.decrease_farming_token_reserve(farming_amount)?;
        self.send_fft_tokens(
            farming_token_id,
            farming_amount,
            destination,
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
        with_locked_rewards: bool,
        entering_epoch: Epoch,
        opt_accept_funds_func: &OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        if reward_amount > &mut 0 {
            if with_locked_rewards {
                self.burn_tokens(reward_token_id, reward_amount);
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
                *reward_token_id = result.token_id;
                *reward_nonce = result.token_nonce;
                *reward_amount = result.amount;
            } else {
                self.send_fft_tokens(
                    reward_token_id,
                    reward_amount,
                    destination,
                    opt_accept_funds_func,
                )?;
            }
        }
        Ok(())
    }

    #[payable("*")]
    #[endpoint(acceptFee)]
    fn accept_fee(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] amount: BigUint,
    ) -> SCResult<()> {
        let reward_token_id = self.reward_token_id().get();
        require!(token_in == reward_token_id, "Bad fee token identifier");
        require!(amount > 0, "Zero amount in");
        self.increase_current_block_fee_storage(&amount);
        Ok(())
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        amount: BigUint,
        attributes_raw: BoxedBytes,
    ) -> SCResult<BigUint> {
        require!(amount > 0, "Zero liquidity input");
        let farm_token_supply = self.get_farm_token_supply();
        require!(farm_token_supply >= amount, "Not enough supply");

        let last_reward_nonce = self.last_reward_block_nonce().get();
        let current_block_nonce = self.blockchain().get_block_nonce();
        let to_be_minted = self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);

        let big_zero = self.types().big_uint_zero();
        let mut fees = self.undistributed_fee_storage().get();
        fees += match self.current_block_fee_storage().get() {
            Some((block_nonce, fee_amount)) => {
                if current_block_nonce > block_nonce {
                    fee_amount
                } else {
                    big_zero
                }
            }
            None => big_zero,
        };

        let reward_increase = to_be_minted + fees;
        let reward_per_share_increase = self.calculate_reward_per_share_increase(&reward_increase);

        let attributes = self.decode_attributes(&attributes_raw)?;
        let future_reward_per_share = self.reward_per_share().get() + reward_per_share_increase;
        let reward = self.calculate_reward(
            &amount,
            &future_reward_per_share,
            &attributes.reward_per_share,
        );

        if self.should_apply_penalty(attributes.entering_epoch) {
            Ok(&reward - &self.get_penalty_amount(&reward))
        } else {
            Ok(reward)
        }
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

    fn increase_farming_token_reserve(&self, amount: &BigUint) {
        let current = self.farming_token_reserve().get();
        self.farming_token_reserve().set(&(&current + amount));
    }

    fn decrease_farming_token_reserve(&self, amount: &BigUint) -> SCResult<()> {
        let current = self.farming_token_reserve().get();
        require!(&current >= amount, "Not enough farming reserve");
        self.farming_token_reserve().set(&(&current - amount));
        Ok(())
    }

    #[view(getFarmingTokenReserve)]
    #[storage_mapper("farming_token_reserve")]
    fn farming_token_reserve(&self) -> SingleValueMapper<BigUint>;
}
