#![no_std]

multiversx_sc::imports!();

use events::RedeemEventArgs;

pub mod common_storage;
pub mod events;
pub mod phase;
pub mod redeem_token;
pub mod user_actions;
pub mod views;

pub type Nonce = u64;
pub type Block = u64;
pub type Epoch = u64;
pub type Timestamp = u64;

const MAX_TOKEN_DECIMALS: u32 = 18;

#[multiversx_sc::contract]
pub trait PriceDiscovery:
    common_storage::CommonStorageModule
    + events::EventsModule
    + phase::PhaseModule
    + redeem_token::RedeemTokenModule
    + user_actions::user_deposit_withdraw::UserDepositWithdrawModule
    + views::ViewsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    /// For explanations regarding what each parameter means, please refer to docs/setup.md
    #[init]
    fn init(
        &self,
        launched_token_id: TokenIdentifier,
        accepted_token_id: EgldOrEsdtTokenIdentifier,
        launched_token_decimals: u32,
        min_launched_tokens: BigUint,
        start_time: Timestamp,
        user_deposit_withdraw_time: Timestamp,
        owner_deposit_withdraw_time: Timestamp,
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

        let current_time = self.blockchain().get_block_timestamp();
        require!(
            current_time < start_time,
            "Start time cannot be in the past"
        );
        require!(
            user_deposit_withdraw_time > 0 && owner_deposit_withdraw_time > 0,
            "Invalid timestamps"
        );

        require!(min_launched_tokens > 0, "Invalid min launched tokens");

        self.launched_token_id().set(launched_token_id);
        self.accepted_token_id().set(accepted_token_id);
        self.start_time().set(start_time);
        self.user_deposit_withdraw_time()
            .set(owner_deposit_withdraw_time);
        self.owner_deposit_withdraw_time()
            .set(owner_deposit_withdraw_time);

        let price_precision = 10u64.pow(launched_token_decimals);
        self.price_precision().set(price_precision);
        self.min_launched_tokens().set(min_launched_tokens);
    }

    #[upgrade]
    fn upgrade(&self) {}

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

        self.emit_redeem_event(RedeemEventArgs {
            // redeem_token_id: &payment_token,: TODO: Change to option
            redeem_token_amount: &payment_amount,
            bought_token_id: &bought_tokens.token_identifier,
            bought_token_amount: &bought_tokens.amount,
        });

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
}
