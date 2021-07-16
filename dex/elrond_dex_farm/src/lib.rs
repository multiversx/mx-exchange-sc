#![no_std]
#![allow(clippy::too_many_arguments)]

pub mod config;
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

const DEFAULT_PENALTY_PERCENT: u8 = 10;
const DEFAULT_MINUMUM_FARMING_EPOCHS: u8 = 3;
const DEFAULT_LOCKED_REWARDS_LIQUIDITY_MUTIPLIER: u8 = 2;
const DEFAULT_TRANSFER_EXEC_GAS_LIMIT: u64 = 35000000;
const DEFAULT_NFT_DEPOSIT_MAX_LEN: usize = 10;

type EnterFarmResultType<BigUint> = GenericTokenAmountPair<BigUint>;
type CompoundRewardsResultType<BigUint> = GenericTokenAmountPair<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiResult2<GenericTokenAmountPair<BigUint>, GenericTokenAmountPair<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiResult2<FftTokenAmountPair<BigUint>, GenericTokenAmountPair<BigUint>>;

#[elrond_wasm_derive::contract]
pub trait Farm:
    rewards::RewardsModule
    + config::ConfigModule
    + token_supply::TokenSupplyModule
    + nft_deposit::NftDepositModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
    + farm_token::FarmTokenModule
    + farm_token_merge::FarmTokenMergeModule
{
    #[proxy]
    fn locked_asset_factory(&self, to: Address) -> sc_locked_asset_factory::Proxy<Self::SendApi>;

    #[init]
    fn init(
        &self,
        router_address: Address,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        locked_asset_factory_address: Address,
        division_safety_constant: Self::BigUint,
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
        self.nft_deposit_max_len()
            .set_if_empty(&DEFAULT_NFT_DEPOSIT_MAX_LEN);

        self.owner().set(&self.blockchain().get_caller());
        self.router_address().set(&router_address);
        self.reward_token_id().set(&reward_token_id);
        self.farming_token_id().set(&farming_token_id);
        self.locked_asset_factory_address()
            .set(&locked_asset_factory_address);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] enter_amount: Self::BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<EnterFarmResultType<Self::BigUint>> {
        self.enter_farm_common(token_in, enter_amount, false, opt_accept_funds_func)
    }

    #[payable("*")]
    #[endpoint(enterFarmAndLockRewards)]
    fn enter_farm_and_lock_rewards(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] enter_amount: Self::BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<EnterFarmResultType<Self::BigUint>> {
        self.enter_farm_common(token_in, enter_amount, true, opt_accept_funds_func)
    }

    fn enter_farm_common(
        &self,
        token_in: TokenIdentifier,
        enter_amount: Self::BigUint,
        with_locked_rewards: bool,
        opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<EnterFarmResultType<Self::BigUint>> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No issued farm token");
        let farming_token_id = self.farming_token_id().get();
        require!(token_in == farming_token_id, "Bad input token");
        require!(enter_amount > 0, "Cannot farm with amount of 0");
        self.increase_farming_token_reserve(&enter_amount);

        let (mut farm_contribution, apr_multiplier) =
            self.get_farm_contribution(&enter_amount, with_locked_rewards);

        let reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards(&reward_token_id);

        let mut attributes = FarmTokenAttributes {
            reward_per_share: self.reward_per_share().get(),
            entering_epoch: self.blockchain().get_block_epoch(),
            apr_multiplier,
            with_locked_rewards,
            initial_farming_amount: enter_amount,
            compounded_reward: 0u64.into(),
            current_farm_amount: farm_contribution.clone(),
        };

        let caller = self.blockchain().get_caller();
        let farm_token_id = self.farm_token_id().get();
        let new_nonce = self.create_farm_tokens_by_merging(
            &mut farm_contribution,
            &farm_token_id,
            &mut attributes,
            &caller,
        )?;
        self.send_nft_tokens(
            &farm_token_id,
            new_nonce,
            &farm_contribution,
            &caller,
            &opt_accept_funds_func,
        )?;

        Ok(GenericTokenAmountPair {
            token_id: farm_token_id,
            token_nonce: new_nonce,
            amount: farm_contribution,
        })
    }

    fn get_farm_contribution(
        &self,
        amount: &Self::BigUint,
        with_locked_rewards: bool,
    ) -> (Self::BigUint, u8) {
        if with_locked_rewards {
            let multiplier = self.locked_rewards_apr_multiplier().get();
            (amount * &(multiplier as u64).into(), multiplier)
        } else {
            (amount.clone(), 1u8)
        }
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
        #[payment_nonce] token_nonce: Nonce,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<ExitFarmResultType<Self::BigUint>> {
        require!(!self.farm_token_id().is_empty(), "No issued farm token");
        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Bad input token");
        require!(amount > 0, "Payment amount cannot be zero");

        let farm_attributes = self.get_farm_attributes(&payment_token_id, token_nonce)?;
        require!(
            !farm_attributes.with_locked_rewards
                || !self.should_apply_penalty(farm_attributes.entering_epoch),
            "Exit too early for lock rewards option"
        );

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
        let mut initial_farming_token_amount = self.rule_of_three(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.initial_farming_amount,
        );
        reward += self.rule_of_three(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        );
        require!(
            initial_farming_token_amount != 0,
            "Farming token amount is zero"
        );

        if self.should_apply_penalty(farm_attributes.entering_epoch) {
            let mut penalty_amount = self.get_penalty_amount(&reward);
            if penalty_amount > 0 {
                self.burn_tokens(&reward_token_id, &penalty_amount);
                reward -= penalty_amount;
            }

            penalty_amount = self.get_penalty_amount(&initial_farming_token_amount);
            if penalty_amount > 0 {
                self.burn_tokens(&farming_token_id, &penalty_amount);
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
            farm_attributes.entering_epoch,
            &opt_accept_funds_func,
        )?;

        Ok((
            FftTokenAmountPair {
                token_id: farming_token_id,
                amount: initial_farming_token_amount,
            },
            GenericTokenAmountPair {
                token_id: reward_token_id,
                token_nonce: reward_nonce,
                amount: reward,
            },
        )
            .into())
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
        #[payment_nonce] token_nonce: Nonce,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<ClaimRewardsResultType<Self::BigUint>> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No issued farm token");
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

        let new_initial_farming_amount = self.rule_of_three(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.initial_farming_amount,
        );
        require!(
            new_initial_farming_amount != 0,
            "Farming token amount is zero"
        );
        let new_compound_reward_amount = self.rule_of_three(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        );

        let mut new_attributes = FarmTokenAttributes {
            reward_per_share: self.reward_per_share().get(),
            entering_epoch: farm_attributes.entering_epoch,
            apr_multiplier: farm_attributes.apr_multiplier,
            with_locked_rewards: farm_attributes.with_locked_rewards,
            initial_farming_amount: new_initial_farming_amount,
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: amount.clone(),
        };

        let caller = self.blockchain().get_caller();
        self.burn_farm_tokens(&payment_token_id, token_nonce, &amount)?;
        let mut farm_amount = amount.clone();
        let new_nonce = self.create_farm_tokens_by_merging(
            &mut farm_amount,
            &farm_token_id,
            &mut new_attributes,
            &caller,
        )?;
        self.send_nft_tokens(
            &farm_token_id,
            new_nonce,
            &amount,
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
            farm_attributes.entering_epoch,
            &opt_accept_funds_func,
        )?;

        Ok((
            GenericTokenAmountPair {
                token_id: farm_token_id,
                token_nonce: new_nonce,
                amount,
            },
            GenericTokenAmountPair {
                token_id: reward_token_id,
                token_nonce: reward_nonce,
                amount: reward,
            },
        )
            .into())
    }

    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_amount] payment_amount: Self::BigUint,
        #[payment_nonce] payment_token_nonce: Nonce,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<CompoundRewardsResultType<Self::BigUint>> {
        require!(self.is_active(), "Not active");
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
        let mut new_farm_contribution =
            &payment_amount + &(&reward * &(farm_attributes.apr_multiplier as u64).into());

        let new_initial_farming_amount = self.rule_of_three(
            &payment_amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.initial_farming_amount,
        );
        require!(
            new_initial_farming_amount != 0,
            "Farming token amount is zero"
        );
        let new_compound_reward_amount = self.rule_of_three(
            &payment_amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        ) + reward;

        let mut new_attributes = FarmTokenAttributes {
            reward_per_share: current_rps,
            entering_epoch: farm_attributes.entering_epoch,
            apr_multiplier: farm_attributes.apr_multiplier,
            with_locked_rewards: farm_attributes.with_locked_rewards,
            initial_farming_amount: new_initial_farming_amount,
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: new_farm_contribution.clone(),
        };

        self.burn_farm_tokens(&farm_token_id, payment_token_nonce, &payment_amount)?;
        let caller = self.blockchain().get_caller();
        let new_nonce = self.create_farm_tokens_by_merging(
            &mut new_farm_contribution,
            &farm_token_id,
            &mut new_attributes,
            &caller,
        )?;
        self.send_nft_tokens(
            &farm_token_id,
            new_nonce,
            &new_farm_contribution,
            &caller,
            &opt_accept_funds_func,
        )?;

        Ok(GenericTokenAmountPair {
            token_id: farm_token_id,
            token_nonce: new_nonce,
            amount: new_farm_contribution,
        })
    }

    fn create_farm_tokens_by_merging(
        &self,
        amount: &mut Self::BigUint,
        token_id: &TokenIdentifier,
        attributes: &mut FarmTokenAttributes<Self::BigUint>,
        caller: &Address,
    ) -> SCResult<Nonce> {
        let current_position_replic = FarmToken {
            token_amount: GenericTokenAmountPair {
                token_id: token_id.clone(),
                token_nonce: 0,
                amount: amount.clone(),
            },
            attributes: attributes.clone(),
        };

        let deposit = self.nft_deposit(caller).get();
        let merged_attributes =
            self.get_merged_farm_token_attributes(&deposit, Some(current_position_replic))?;
        self.burn_deposit_tokens(caller, &deposit);

        *amount = merged_attributes.current_farm_amount.clone();
        *attributes = merged_attributes.clone();
        Ok(self.create_farm_tokens(amount, token_id, &merged_attributes))
    }

    fn send_back_farming_tokens(
        &self,
        farming_token_id: &TokenIdentifier,
        farming_amount: &Self::BigUint,
        destination: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
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
        reward_amount: &mut Self::BigUint,
        destination: &Address,
        with_locked_rewards: bool,
        entering_epoch: Epoch,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
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
        #[payment_amount] amount: Self::BigUint,
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
        amount: Self::BigUint,
        attributes_raw: BoxedBytes,
    ) -> SCResult<Self::BigUint> {
        require!(amount > 0, "Zero liquidity input");
        let farm_token_supply = self.get_farm_token_supply();
        require!(farm_token_supply >= amount, "Not enough supply");

        let last_reward_nonce = self.last_reward_block_nonce().get();
        let current_block_nonce = self.blockchain().get_block_nonce();
        let to_be_minted = self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);

        let big_zero = 0u64.into();
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
    fn get_penalty_amount(&self, amount: &Self::BigUint) -> Self::BigUint {
        amount * &(self.penalty_percent().get() as u64).into() / 100u64.into()
    }

    fn increase_farming_token_reserve(&self, amount: &Self::BigUint) {
        let current = self.farming_token_reserve().get();
        self.farming_token_reserve().set(&(&current + amount));
    }

    fn decrease_farming_token_reserve(&self, amount: &Self::BigUint) -> SCResult<()> {
        let current = self.farming_token_reserve().get();
        require!(&current >= amount, "Not enough farming reserve");
        self.farming_token_reserve().set(&(&current - amount));
        Ok(())
    }

    #[view(getFarmingTokenReserve)]
    #[storage_mapper("farming_token_reserve")]
    fn farming_token_reserve(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;
}
