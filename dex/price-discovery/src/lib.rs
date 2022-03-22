#![no_std]

use crate::{
    common_storage::MAX_PERCENTAGE,
    redeem_token::{ACCEPTED_TOKEN_REDEEM_NONCE, LAUNCHED_TOKEN_REDEEM_NONCE},
};

elrond_wasm::imports!();

pub mod common_storage;
pub mod create_pool;
pub mod events;
pub mod phase;
pub mod redeem_token;

const INVALID_PAYMENT_ERR_MSG: &[u8] = b"Invalid payment token";
pub const MIN_PRICE_PRECISION: u64 = 1_000_000_000_000_000_000;

pub struct RewardsPair<M: ManagedTypeApi> {
    pub lp_tokens_amount: BigUint<M>,
    pub extra_rewards_amount: BigUint<M>,
}

#[elrond_wasm::contract]
pub trait PriceDiscovery:
    common_storage::CommonStorageModule
    + create_pool::CreatePoolModule
    + events::EventsModule
    + phase::PhaseModule
    + redeem_token::RedeemTokenModule
{
    /// For explanations regarding what each parameter means, please refer to docs/setup.md
    #[init]
    fn init(
        &self,
        launched_token_id: TokenIdentifier,
        accepted_token_id: TokenIdentifier,
        extra_rewards_token_id: TokenIdentifier,
        min_launched_token_price: BigUint,
        start_block: u64,
        no_limit_phase_duration_blocks: u64,
        linear_penalty_phase_duration_blocks: u64,
        fixed_penalty_phase_duration_blocks: u64,
        unbond_period_epochs: u64,
        penalty_min_percentage: BigUint,
        penalty_max_percentage: BigUint,
        fixed_penalty_percentage: BigUint,
        #[var_args] opt_extra_rewards_token_nonce: OptionalValue<u64>,
    ) {
        /* Disabled until the validate token ID function is activated

        require!(
            launched_token_id.is_valid_esdt_identifier(),
            "Invalid launched token ID"
        );
        require!(
            accepted_token_id.is_egld() || accepted_token_id.is_valid_esdt_identifier(),
            "Invalid payment token ID"
        );
        require!(
            extra_rewards_token_id.is_egld() || extra_rewards_token_id.is_valid_esdt_identifier(),
            "Invalid extra rewards token ID"
        );

        */
        require!(
            launched_token_id != accepted_token_id,
            "Launched and accepted token must be different"
        );

        let current_block = self.blockchain().get_block_nonce();
        require!(
            current_block < start_block,
            "Start block cannot be in the past"
        );

        let end_block = start_block
            + no_limit_phase_duration_blocks
            + linear_penalty_phase_duration_blocks
            + fixed_penalty_phase_duration_blocks;

        require!(
            penalty_min_percentage <= penalty_max_percentage,
            "Min percentage higher than max percentage"
        );
        require!(
            penalty_max_percentage < MAX_PERCENTAGE,
            "Max percentage higher than 100%"
        );
        require!(
            fixed_penalty_percentage < MAX_PERCENTAGE,
            "Fixed percentage higher than 100%"
        );

        if let OptionalValue::Some(nonce) = opt_extra_rewards_token_nonce {
            self.extra_rewards_token_nonce().set(&nonce);
        }

        self.launched_token_id().set(&launched_token_id);
        self.accepted_token_id().set(&accepted_token_id);
        self.extra_rewards_token_id().set(&extra_rewards_token_id);
        self.min_launched_token_price()
            .set(&min_launched_token_price);
        self.start_block().set(&start_block);
        self.end_block().set(&end_block);

        self.no_limit_phase_duration_blocks()
            .set(&no_limit_phase_duration_blocks);
        self.linear_penalty_phase_duration_blocks()
            .set(&linear_penalty_phase_duration_blocks);
        self.fixed_penalty_phase_duration_blocks()
            .set(&fixed_penalty_phase_duration_blocks);
        self.unbond_period_epochs().set(&unbond_period_epochs);
        self.penalty_min_percentage().set(&penalty_min_percentage);
        self.penalty_max_percentage().set(&penalty_max_percentage);
        self.fixed_penalty_percentage()
            .set(&fixed_penalty_percentage);
    }

    /// Extra rewards that will be given to users that contributed to the pool, defined by
    /// extra_rewards_token_id. Can be deposited by anyone.
    #[payable("*")]
    #[endpoint(depositExtraRewards)]
    fn deposit_extra_rewards(&self) {
        self.require_dex_address_set();

        let phase = self.get_current_phase();
        self.require_deposit_extra_rewards_allowed(&phase);

        let (payment_token, payment_nonce, payment_amount) = self.call_value().payment_as_tuple();
        let extra_rewards_token_id = self.extra_rewards_token_id().get();
        let extra_rewards_token_nonce = self.extra_rewards_token_nonce().get();
        require!(
            payment_token == extra_rewards_token_id && payment_nonce == extra_rewards_token_nonce,
            INVALID_PAYMENT_ERR_MSG
        );

        self.increase_balance(self.extra_rewards_balance(), &payment_amount);

        self.emit_deposit_extra_rewards_event(extra_rewards_token_id, payment_amount);
    }

    /// Users can deposit either launched_token or accepted_token.
    /// They will receive an SFT that can be used to withdraw said tokens
    #[payable("*")]
    #[endpoint]
    fn deposit(&self) {
        self.require_dex_address_set();

        let phase = self.get_current_phase();
        self.require_deposit_allowed(&phase);

        let (payment_amount, payment_token) = self.call_value().payment_token_pair();
        let accepted_token_id = self.accepted_token_id().get();
        let launched_token_id = self.launched_token_id().get();
        let redeem_token_id = self.redeem_token_id().get();
        let (redeem_token_nonce, balance_mapper) = if payment_token == accepted_token_id {
            (ACCEPTED_TOKEN_REDEEM_NONCE, self.accepted_token_balance())
        } else if payment_token == launched_token_id {
            (LAUNCHED_TOKEN_REDEEM_NONCE, self.launched_token_balance())
        } else {
            sc_panic!(INVALID_PAYMENT_ERR_MSG);
        };

        self.increase_balance(balance_mapper, &payment_amount);

        let caller = self.blockchain().get_caller();
        self.mint_and_send_redeem_token(&caller, redeem_token_nonce, &payment_amount);

        let current_price = self.get_launched_token_price_over_min_price();

        self.emit_deposit_event(
            payment_token,
            payment_amount.clone(),
            redeem_token_id,
            redeem_token_nonce,
            payment_amount,
            current_price,
            phase,
        );
    }

    /// Deposit SFTs received after deposit to withdraw the initially deposited tokens.
    /// Depending on the current Phase, a penalty may be applied and only a part
    /// of the initial tokens will be received.
    #[payable("*")]
    #[endpoint]
    fn withdraw(&self) {
        self.require_dex_address_set();

        let phase = self.get_current_phase();
        self.require_withdraw_allowed(&phase);

        let (payment_token, payment_nonce, payment_amount) = self.call_value().payment_as_tuple();
        let redeem_token_id = self.redeem_token_id().get();
        require!(payment_token == redeem_token_id, INVALID_PAYMENT_ERR_MSG);

        let (refund_token_id, balance_mapper) = match payment_nonce {
            LAUNCHED_TOKEN_REDEEM_NONCE => (
                self.launched_token_id().get(),
                self.launched_token_balance(),
            ),
            ACCEPTED_TOKEN_REDEEM_NONCE => (
                self.accepted_token_id().get(),
                self.accepted_token_balance(),
            ),
            _ => sc_panic!(INVALID_PAYMENT_ERR_MSG),
        };

        self.burn_redeem_token(payment_nonce, &payment_amount);

        let penalty_percentage = phase.get_penalty_percentage();
        let penalty_amount = &payment_amount * &penalty_percentage / MAX_PERCENTAGE;

        let caller = self.blockchain().get_caller();
        let withdraw_amount = &payment_amount - &penalty_amount;
        if withdraw_amount > 0 {
            self.decrease_balance(balance_mapper, &withdraw_amount);

            self.send()
                .direct(&caller, &refund_token_id, 0, &withdraw_amount, &[]);
        }

        let current_price = self.get_launched_token_price_over_min_price();

        self.emit_withdraw_event(
            refund_token_id,
            withdraw_amount,
            payment_token,
            payment_nonce,
            payment_amount,
            current_price,
            phase,
        );
    }

    /// After the liquidity pool has been created and the LP tokens received,
    /// users can withdraw their fair share of the LP tokens by depositing their SFTs
    /// and a share of the extra rewards
    #[payable("*")]
    #[endpoint]
    fn redeem(&self) {
        self.require_redeem_allowed();

        let (payment_token, payment_nonce, payment_amount) = self.call_value().payment_as_tuple();
        let redeem_token_id = self.redeem_token_id().get();
        require!(payment_token == redeem_token_id, INVALID_PAYMENT_ERR_MSG);

        let rewards = self.compute_rewards(payment_nonce, &payment_amount);
        self.burn_redeem_token_without_supply_decrease(payment_nonce, &payment_amount);

        let caller = self.blockchain().get_caller();
        if rewards.lp_tokens_amount > 0 {
            let lp_token_id = self.lp_token_id().get();
            self.send()
                .direct(&caller, &lp_token_id, 0, &rewards.lp_tokens_amount, &[]);
        }
        if rewards.extra_rewards_amount > 0 {
            self.decrease_balance(self.extra_rewards_balance(), &rewards.extra_rewards_amount);

            let extra_rewards_token_id = self.extra_rewards_token_id().get();
            let extra_rewards_token_nonce = self.extra_rewards_token_nonce().get();
            self.send().direct(
                &caller,
                &extra_rewards_token_id,
                extra_rewards_token_nonce,
                &rewards.extra_rewards_amount,
                &[],
            );
        }

        let total_lp_tokens = self.total_lp_tokens_received().get();
        let remaining_lp_tokens = self.lp_tokens_claimed().update(|total_claimed| {
            *total_claimed += &rewards.lp_tokens_amount;
            &total_lp_tokens - &*total_claimed
        });

        self.emit_redeem_event(
            payment_token,
            payment_nonce,
            payment_amount,
            self.lp_token_id().get(),
            rewards.lp_tokens_amount,
            remaining_lp_tokens,
            total_lp_tokens,
            self.extra_rewards_token_id().get(),
            rewards.extra_rewards_amount,
        )
    }

    // private

    fn compute_rewards(
        &self,
        redeem_token_nonce: u64,
        redeem_token_amount: &BigUint,
    ) -> RewardsPair<Self::Api> {
        let total_lp_tokens = self.total_lp_tokens_received().get();
        let redeem_token_supply = self
            .redeem_token_total_circulating_supply(redeem_token_nonce)
            .get();

        let lp_tokens_amount = &total_lp_tokens * redeem_token_amount / redeem_token_supply / 2u32;

        let total_extra_rewards = self.total_extra_rewards_tokens().get();
        let extra_rewards_amount = &total_extra_rewards * &lp_tokens_amount / total_lp_tokens;

        RewardsPair {
            lp_tokens_amount,
            extra_rewards_amount,
        }
    }

    fn require_redeem_allowed(&self) {
        let pool_creation_epoch = self.pool_creation_epoch().get();
        require!(pool_creation_epoch > 0, "Liquidity Pool not created yet");

        let unbond_epochs = self.unbond_period_epochs().get();
        let redeem_activation_epoch = pool_creation_epoch + unbond_epochs;
        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            current_epoch >= redeem_activation_epoch,
            "Unbond period not finished yet"
        );
    }

    fn get_launched_token_price_over_min_price(&self) -> BigUint {
        let min_price = self.min_launched_token_price().get();
        let launched_token_balance = self.launched_token_balance().get();
        let accepted_token_balance = self.accepted_token_balance().get();

        if accepted_token_balance == 0 {
            return accepted_token_balance;
        }

        require!(launched_token_balance > 0, "No launched tokens available");

        let current_price = accepted_token_balance * MIN_PRICE_PRECISION / launched_token_balance;
        require!(current_price >= min_price, "Launched token below min price");

        current_price
    }

    fn increase_balance(&self, mapper: SingleValueMapper<BigUint>, amount: &BigUint) {
        mapper.update(|b| *b += amount);
    }

    fn decrease_balance(&self, mapper: SingleValueMapper<BigUint>, amount: &BigUint) {
        mapper.update(|b| *b -= amount);
    }

    #[view(getMinLaunchedTokenPrice)]
    #[storage_mapper("minLaunchedTokenPrice")]
    fn min_launched_token_price(&self) -> SingleValueMapper<BigUint>;

    #[view(getLpTokensClaimed)]
    #[storage_mapper("lpTokensClaimed")]
    fn lp_tokens_claimed(&self) -> SingleValueMapper<BigUint>;
}
