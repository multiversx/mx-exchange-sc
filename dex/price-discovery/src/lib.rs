#![no_std]

multiversx_sc::imports!();

use crate::{
    common_storage::MAX_PERCENTAGE,
    redeem_token::{ACCEPTED_TOKEN_REDEEM_NONCE, LAUNCHED_TOKEN_REDEEM_NONCE},
};

pub mod common_storage;
pub mod events;
pub mod phase;
pub mod redeem_token;

static INVALID_PAYMENT_ERR_MSG: &[u8] = b"Invalid payment token";
static BELOW_MIN_PRICE_ERR_MSG: &[u8] = b"Launched token below min price";
const MAX_TOKEN_DECIMALS: u32 = 18;

#[multiversx_sc::contract]
pub trait PriceDiscovery:
    common_storage::CommonStorageModule
    + events::EventsModule
    + locking_module::locking_module::LockingModule
    + phase::PhaseModule
    + redeem_token::RedeemTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    /// For explanations regarding what each parameter means, please refer to docs/setup.md
    #[init]
    fn init(
        &self,
        launched_token_id: TokenIdentifier,
        accepted_token_id: EgldOrEsdtTokenIdentifier,
        launched_token_decimals: u32,
        min_launched_token_price: BigUint,
        start_block: u64,
        no_limit_phase_duration_blocks: u64,
        linear_penalty_phase_duration_blocks: u64,
        fixed_penalty_phase_duration_blocks: u64,
        unlock_epoch: u64,
        penalty_min_percentage: BigUint,
        penalty_max_percentage: BigUint,
        fixed_penalty_percentage: BigUint,
        locking_sc_address: ManagedAddress,
    ) {
        require!(
            launched_token_id.is_valid_esdt_identifier(),
            "Invalid launched token ID"
        );
        require!(accepted_token_id.is_valid(), "Invalid payment token ID");
        require!(
            accepted_token_id != launched_token_id,
            "Launched and accepted token must be different"
        );

        require!(
            launched_token_decimals <= MAX_TOKEN_DECIMALS,
            "Launched token has too many decimals"
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

        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            unlock_epoch > current_epoch,
            "Unlock epoch cannot be in the past"
        );

        self.launched_token_id().set(&launched_token_id);
        self.accepted_token_id().set(&accepted_token_id);
        self.start_block().set(start_block);
        self.end_block().set(end_block);
        self.unlock_epoch().set(unlock_epoch);

        let price_precision = 10u64.pow(launched_token_decimals);
        self.price_precision().set(price_precision);
        self.min_launched_token_price()
            .set(&min_launched_token_price);

        self.no_limit_phase_duration_blocks()
            .set(no_limit_phase_duration_blocks);
        self.linear_penalty_phase_duration_blocks()
            .set(linear_penalty_phase_duration_blocks);
        self.fixed_penalty_phase_duration_blocks()
            .set(fixed_penalty_phase_duration_blocks);
        self.penalty_min_percentage().set(&penalty_min_percentage);
        self.penalty_max_percentage().set(&penalty_max_percentage);
        self.fixed_penalty_percentage()
            .set(&fixed_penalty_percentage);

        self.set_locking_sc_address(locking_sc_address);
    }

    /// Users can deposit either launched_token or accepted_token.
    /// They will receive an SFT that can be used to withdraw said tokens
    #[payable("*")]
    #[endpoint]
    fn deposit(&self) -> EsdtTokenPayment<Self::Api> {
        let phase = self.get_current_phase();
        self.require_deposit_allowed(&phase);

        let (payment_token, payment_amount) = self.call_value().egld_or_single_fungible_esdt();
        let accepted_token_id = self.accepted_token_id().get();
        let launched_token_id = self.launched_token_id().get();
        let (redeem_token_nonce, balance_mapper) = if payment_token == accepted_token_id {
            (ACCEPTED_TOKEN_REDEEM_NONCE, self.accepted_token_balance())
        } else if payment_token == launched_token_id {
            (LAUNCHED_TOKEN_REDEEM_NONCE, self.launched_token_balance())
        } else {
            sc_panic!(INVALID_PAYMENT_ERR_MSG);
        };

        self.increase_balance(balance_mapper, &payment_amount);

        let current_price = self.calculate_price();
        let min_price = self.min_launched_token_price().get();
        require!(
            current_price == 0 || current_price >= min_price || payment_token == accepted_token_id,
            BELOW_MIN_PRICE_ERR_MSG
        );

        let caller = self.blockchain().get_caller();
        let payment_result =
            self.mint_and_send_redeem_token(&caller, redeem_token_nonce, payment_amount.clone());

        self.emit_deposit_event(
            payment_token,
            payment_amount.clone(),
            payment_result.token_identifier.clone(),
            redeem_token_nonce,
            payment_amount,
            current_price,
            phase,
        );

        payment_result
    }

    /// Deposit SFTs received after deposit to withdraw the initially deposited tokens.
    /// Depending on the current Phase, a penalty may be applied and only a part
    /// of the initial tokens will be received.
    #[payable("*")]
    #[endpoint]
    fn withdraw(&self) -> EgldOrEsdtTokenPayment<Self::Api> {
        let phase = self.get_current_phase();
        self.require_withdraw_allowed(&phase);

        let (payment_token, payment_nonce, payment_amount) =
            self.call_value().single_esdt().into_tuple();
        let redeem_token_id = self.redeem_token().get_token_id();
        require!(payment_token == redeem_token_id, INVALID_PAYMENT_ERR_MSG);

        let (refund_token_id, balance_mapper) = match payment_nonce {
            LAUNCHED_TOKEN_REDEEM_NONCE => (
                EgldOrEsdtTokenIdentifier::esdt(self.launched_token_id().get()),
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
        let withdraw_amount = &payment_amount - &penalty_amount;

        self.decrease_balance(balance_mapper, &withdraw_amount);

        let current_price = self.calculate_price();
        let min_price = self.min_launched_token_price().get();
        require!(current_price >= min_price, BELOW_MIN_PRICE_ERR_MSG);

        let caller = self.blockchain().get_caller();
        self.send()
            .direct(&caller, &refund_token_id, 0, &withdraw_amount);

        self.emit_withdraw_event(
            refund_token_id.clone(),
            withdraw_amount.clone(),
            payment_token,
            payment_nonce,
            payment_amount,
            current_price,
            phase,
        );

        EgldOrEsdtTokenPayment::new(refund_token_id, 0, withdraw_amount)
    }

    /// After all phases have ended,
    /// users can withdraw their fair share of either accepted or launched tokens,
    /// depending on which token they deposited initially.
    /// Users that deposited accepted tokens will receive Locked launched tokens.
    /// Users that deposited launched tokens will receive Locked accepted tokens.
    /// The users can unlock said tokens at the configured unlock_epoch,
    /// through the SC at locking_sc_address
    #[payable("*")]
    #[endpoint]
    fn redeem(&self) -> EgldOrEsdtTokenPayment<Self::Api> {
        let phase = self.get_current_phase();
        self.require_redeem_allowed(&phase);

        let (payment_token, payment_nonce, payment_amount) =
            self.call_value().single_esdt().into_tuple();
        let redeem_token_id = self.redeem_token().get_token_id();
        require!(payment_token == redeem_token_id, INVALID_PAYMENT_ERR_MSG);

        let bought_tokens = self.compute_bought_tokens(payment_nonce, &payment_amount);
        self.burn_redeem_token_without_supply_decrease(payment_nonce, &payment_amount);

        if bought_tokens.amount > 0 {
            let caller = self.blockchain().get_caller();
            let _ = self.lock_tokens_and_forward(
                caller,
                bought_tokens.token_identifier.clone(),
                bought_tokens.amount.clone(),
            );
        }

        self.emit_redeem_event(
            payment_token,
            payment_nonce,
            payment_amount,
            bought_tokens.token_identifier.clone(),
            bought_tokens.amount.clone(),
        );

        bought_tokens
    }

    // private

    fn compute_bought_tokens(
        &self,
        redeem_token_nonce: u64,
        redeem_token_amount: &BigUint,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        let redeem_token_supply = self
            .redeem_token_total_circulating_supply(redeem_token_nonce)
            .get();

        // users that deposited accepted tokens get launched tokens, and vice-versa
        let (token_id, total_token_supply) = match redeem_token_nonce {
            ACCEPTED_TOKEN_REDEEM_NONCE => (
                EgldOrEsdtTokenIdentifier::esdt(self.launched_token_id().get()),
                self.launched_token_balance().get(),
            ),
            LAUNCHED_TOKEN_REDEEM_NONCE => (
                self.accepted_token_id().get(),
                self.accepted_token_balance().get(),
            ),
            _ => sc_panic!(INVALID_PAYMENT_ERR_MSG),
        };
        let reward_amount = total_token_supply * redeem_token_amount / redeem_token_supply;

        EgldOrEsdtTokenPayment::new(token_id, 0, reward_amount)
    }

    #[view(getCurrentPrice)]
    fn calculate_price(&self) -> BigUint {
        let launched_token_balance = self.launched_token_balance().get();
        let accepted_token_balance = self.accepted_token_balance().get();

        require!(launched_token_balance > 0, "No launched tokens available");

        let price_precision = self.price_precision().get();
        accepted_token_balance * price_precision / launched_token_balance
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

    #[view(getPricePrecision)]
    #[storage_mapper("pricePrecision")]
    fn price_precision(&self) -> SingleValueMapper<u64>;
}
