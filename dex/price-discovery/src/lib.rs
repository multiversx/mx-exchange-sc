#![no_std]

multiversx_sc::imports!();

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
    + user_actions::owner_deposit_withdraw::OwnerDepositWithdrawModule
    + user_actions::redeem::RedeemModule
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
}
