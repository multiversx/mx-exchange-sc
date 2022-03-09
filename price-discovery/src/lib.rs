#![no_std]

use crate::{
    common_storage::MAX_PERCENTAGE,
    redeem_token::{ACCEPTED_TOKEN_REDEEM_NONCE, LAUNCHED_TOKEN_REDEEM_NONCE},
};

elrond_wasm::imports!();

pub mod common_storage;
pub mod create_pool;
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
    + phase::PhaseModule
    + redeem_token::RedeemTokenModule
{
    #[init]
    fn init(
        &self,
        launched_token_id: TokenIdentifier,
        accepted_token_id: TokenIdentifier,
        extra_rewards_token_id: TokenIdentifier,
        min_launched_token_price: BigUint,
        start_block: u64,
        end_block: u64,
        no_limit_phase_duration_blocks: u64,
        linear_penalty_phase_duration_blocks: u64,
        fixed_penalty_phase_duration_blocks: u64,
        unbond_period_epochs: u64,
        penalty_min_percentage: BigUint,
        penalty_max_percentage: BigUint,
        fixed_penalty_percentage: BigUint,
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

        self.check_valid_init_periods(
            start_block,
            end_block,
            no_limit_phase_duration_blocks,
            linear_penalty_phase_duration_blocks,
            fixed_penalty_phase_duration_blocks,
        );

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

    #[inline]
    fn check_valid_init_periods(
        &self,
        start_block: u64,
        end_block: u64,
        no_limit_phase_duration_blocks: u64,
        linear_penalty_phase_duration_blocks: u64,
        fixed_penalty_phase_duration_blocks: u64,
    ) {
        let current_block = self.blockchain().get_block_nonce();
        require!(
            current_block < start_block,
            "Start block cannot be in the past"
        );
        require!(current_block < end_block, "End epoch cannot be in the past");
        require!(
            start_block < end_block,
            "Start epoch must be before end epoch"
        );

        let block_diff = end_block - start_block;
        let phases_total_duration = no_limit_phase_duration_blocks
            + linear_penalty_phase_duration_blocks
            + fixed_penalty_phase_duration_blocks;
        require!(
            phases_total_duration <= block_diff,
            "Phase durations last more than the whole start to end period"
        );
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(depositExtraRewards)]
    fn deposit_extra_rewards(&self) {
        self.require_dex_address_set();

        let phase = self.get_current_phase();
        self.require_deposit_extra_rewards_allowed(&phase);

        let payment_token = self.call_value().token();
        let extra_rewards_token_id = self.extra_rewards_token_id().get();
        require!(
            payment_token == extra_rewards_token_id,
            INVALID_PAYMENT_ERR_MSG
        );
    }

    #[payable("*")]
    #[endpoint]
    fn deposit(&self) {
        self.require_dex_address_set();

        let phase = self.get_current_phase();
        self.require_deposit_allowed(&phase);

        let (payment_amount, payment_token) = self.call_value().payment_token_pair();
        let accepted_token_id = self.accepted_token_id().get();
        let launched_token_id = self.launched_token_id().get();
        let redeem_token_nonce = if payment_token == accepted_token_id {
            ACCEPTED_TOKEN_REDEEM_NONCE
        } else if payment_token == launched_token_id {
            LAUNCHED_TOKEN_REDEEM_NONCE
        } else {
            sc_panic!(INVALID_PAYMENT_ERR_MSG);
        };

        let caller = self.blockchain().get_caller();
        self.mint_and_send_redeem_token(&caller, redeem_token_nonce, &payment_amount);

        self.require_launched_token_over_min_price();
    }

    #[payable("*")]
    #[endpoint]
    fn withdraw(&self) {
        self.require_dex_address_set();

        let phase = self.get_current_phase();
        self.require_withdraw_allowed(&phase);

        let (payment_amount, payment_token) = self.call_value().payment_token_pair();
        let payment_nonce = self.call_value().esdt_token_nonce();

        let redeem_token_id = self.redeem_token_id().get();
        require!(payment_token == redeem_token_id, INVALID_PAYMENT_ERR_MSG);

        let refund_token_id = match payment_nonce {
            LAUNCHED_TOKEN_REDEEM_NONCE => self.launched_token_id().get(),
            ACCEPTED_TOKEN_REDEEM_NONCE => self.accepted_token_id().get(),
            _ => sc_panic!(INVALID_PAYMENT_ERR_MSG),
        };

        self.burn_redeem_token(payment_nonce, &payment_amount);

        let penalty_percentage = phase.to_penalty_percentage();
        let penalty_amount = &payment_amount * &penalty_percentage / MAX_PERCENTAGE;
        if penalty_amount > 0 {
            self.accumulated_penalty(payment_nonce)
                .update(|p| *p += &penalty_amount);
        }

        let caller = self.blockchain().get_caller();
        let withdraw_amount = payment_amount - penalty_amount;
        if withdraw_amount > 0 {
            self.send()
                .direct(&caller, &refund_token_id, 0, &withdraw_amount, &[]);
        }

        self.require_launched_token_over_min_price();
    }

    #[payable("*")]
    #[endpoint]
    fn redeem(&self) {
        self.require_redeem_allowed();

        let (payment_amount, payment_token) = self.call_value().payment_token_pair();
        let payment_nonce = self.call_value().esdt_token_nonce();

        let redeem_token_id = self.redeem_token_id().get();
        require!(payment_token == redeem_token_id, INVALID_PAYMENT_ERR_MSG);

        let rewards = self.compute_rewards(payment_nonce, &payment_amount);
        self.burn_redeem_token(payment_nonce, &payment_amount);

        let caller = self.blockchain().get_caller();
        if rewards.lp_tokens_amount > 0 {
            let lp_token_id = self.lp_token_id().get();
            self.send()
                .direct(&caller, &lp_token_id, 0, &rewards.lp_tokens_amount, &[]);
        }
        if rewards.extra_rewards_amount > 0 {
            let extra_rewards_token_id = self.extra_rewards_token_id().get();
            self.send().direct(
                &caller,
                &extra_rewards_token_id,
                0,
                &rewards.extra_rewards_amount,
                &[],
            );
        }
    }

    // private

    fn compute_rewards(
        &self,
        redeem_token_nonce: u64,
        redeem_token_amount: &BigUint,
    ) -> RewardsPair<Self::Api> {
        let total_lp_tokens = self.total_claimable_lp_tokens().get();
        let base_lp_amount = match redeem_token_nonce {
            LAUNCHED_TOKEN_REDEEM_NONCE => {
                let launched_token_final_amount = self.launched_token_final_amount().get();
                redeem_token_amount * &total_lp_tokens / launched_token_final_amount / 2u32
            }
            ACCEPTED_TOKEN_REDEEM_NONCE => {
                let accepted_token_final_amount = self.accepted_token_final_amount().get();
                redeem_token_amount * &total_lp_tokens / accepted_token_final_amount / 2u32
            }
            _ => BigUint::zero(),
        };

        let total_extra_lp_tokens = self.extra_lp_tokens().get();
        let user_bonus_lp_tokens = (&total_extra_lp_tokens * &base_lp_amount) / &total_lp_tokens;

        let total_extra_rewards = self.extra_rewards_final_amount().get();
        let user_extra_rewards = &total_extra_rewards * &base_lp_amount / total_lp_tokens;

        RewardsPair {
            lp_tokens_amount: base_lp_amount + user_bonus_lp_tokens,
            extra_rewards_amount: user_extra_rewards,
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

    fn require_launched_token_over_min_price(&self) {
        let launched_token_id = self.launched_token_id().get();
        let accepted_token_id = self.accepted_token_id().get();

        let min_price = self.min_launched_token_price().get();
        let launched_token_balance = self.blockchain().get_sc_balance(&launched_token_id, 0);
        let accepted_token_balance = self.blockchain().get_sc_balance(&accepted_token_id, 0);

        if accepted_token_balance == 0 {
            return;
        }

        require!(launched_token_balance > 0, "No launched tokens available");
        require!(
            accepted_token_balance * MIN_PRICE_PRECISION / launched_token_balance >= min_price,
            "Launched token below min price"
        );
    }

    #[view(getMinLaunchedTokenPrice)]
    #[storage_mapper("minLaunchedTokenPrice")]
    fn min_launched_token_price(&self) -> SingleValueMapper<BigUint>;
}
