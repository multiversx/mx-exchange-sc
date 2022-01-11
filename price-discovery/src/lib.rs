#![no_std]

use crate::redeem_token::{ACCEPTED_TOKEN_REDEEM_NONCE, LAUNCHED_TOKEN_REDEEM_NONCE};

elrond_wasm::imports!();

pub mod common_storage;
pub mod create_pool;
pub mod redeem_token;

const INVALID_PAYMENT_ERR_MSG: &[u8] = b"Invalid payment token";

#[elrond_wasm::contract]
pub trait PriceDiscovery:
    common_storage::CommonStorageModule
    + create_pool::CreatePoolModule
    + redeem_token::RedeemTokenModule
{
    #[init]
    fn init(
        &self,
        dex_sc_address: ManagedAddress,
        launched_token_id: TokenIdentifier,
        accepted_token_id: TokenIdentifier,
        start_epoch: u64,
        end_epoch: u64,
    ) -> SCResult<()> {
        require!(
            self.blockchain().is_smart_contract(&dex_sc_address),
            "Invalid DEX SC address"
        );
        require!(
            launched_token_id.is_valid_esdt_identifier(),
            "Invalid launched token ID"
        );
        require!(
            accepted_token_id.is_egld() || accepted_token_id.is_valid_esdt_identifier(),
            "Invalid payment token ID"
        );

        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            current_epoch < start_epoch,
            "Start epoch cannot be in the past"
        );
        require!(current_epoch < end_epoch, "End epoch cannot be in the past");
        require!(
            start_epoch < end_epoch,
            "Start epoch must be before end epoch"
        );

        self.dex_sc_address().set(&dex_sc_address);
        self.launched_token_id().set(&launched_token_id);
        self.accepted_token_id().set(&accepted_token_id);
        self.start_epoch().set(&start_epoch);
        self.end_epoch().set(&end_epoch);

        Ok(())
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(depositInitialTokens)]
    fn deposit_initial_tokens(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_amount] payment_amount: BigUint,
    ) -> SCResult<()> {
        let current_epoch = self.blockchain().get_block_epoch();
        let start_epoch = self.start_epoch().get();
        require!(
            current_epoch < start_epoch,
            "May only deposit before start epoch"
        );

        let launched_token_id = self.launched_token_id().get();
        require!(payment_token == launched_token_id, INVALID_PAYMENT_ERR_MSG);

        let caller = self.blockchain().get_caller();
        self.mint_and_send_redeem_token(&caller, LAUNCHED_TOKEN_REDEEM_NONCE, &payment_amount);

        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn deposit(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_amount] payment_amount: BigUint,
    ) -> SCResult<()> {
        self.require_active()?;

        let accepted_token_id = self.accepted_token_id().get();
        require!(payment_token == accepted_token_id, INVALID_PAYMENT_ERR_MSG);

        let caller = self.blockchain().get_caller();
        self.mint_and_send_redeem_token(&caller, ACCEPTED_TOKEN_REDEEM_NONCE, &payment_amount);

        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn withdraw(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_nonce] payment_nonce: u64,
        #[payment_amount] payment_amount: BigUint,
    ) -> SCResult<()> {
        self.require_active()?;

        let redeem_token_id = self.redeem_token_id().get();
        require!(
            payment_token == redeem_token_id && payment_nonce == ACCEPTED_TOKEN_REDEEM_NONCE,
            INVALID_PAYMENT_ERR_MSG
        );

        self.burn_redeem_token(ACCEPTED_TOKEN_REDEEM_NONCE, &payment_amount);

        let caller = self.blockchain().get_caller();
        let accepted_token_id = self.accepted_token_id().get();
        self.send()
            .direct(&caller, &accepted_token_id, 0, &payment_amount, &[]);

        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn redeem(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_nonce] payment_nonce: u64,
        #[payment_amount] payment_amount: BigUint,
    ) -> SCResult<()> {
        self.require_deposit_period_ended()?;
        require!(!self.lp_token_id().is_empty(), "Pool not created yet");

        let redeem_token_id = self.redeem_token_id().get();
        require!(payment_token == redeem_token_id, INVALID_PAYMENT_ERR_MSG);

        self.burn_redeem_token(payment_nonce, &payment_amount);

        let lp_token_amount = self.compute_lp_amount_to_send(payment_nonce, payment_amount);
        require!(lp_token_amount > 0u32, "Nothing to redeem");

        let caller = self.blockchain().get_caller();
        let lp_token_id = self.lp_token_id().get();
        self.send()
            .direct(&caller, &lp_token_id, 0, &lp_token_amount, &[]);

        Ok(())
    }

    // private

    fn require_active(&self) -> SCResult<()> {
        let current_epoch = self.blockchain().get_block_epoch();
        let start_epoch = self.start_epoch().get();
        let end_epoch = self.end_epoch().get();
        require!(
            start_epoch <= current_epoch,
            "Deposit period not started yet"
        );
        require!(current_epoch < end_epoch, "Deposit period ended");

        let launched_token_id = self.launched_token_id().get();
        let current_launched_token_balance =
            self.blockchain().get_sc_balance(&launched_token_id, 0);
        require!(
            current_launched_token_balance > 0,
            "Launched tokens not deposited"
        );

        Ok(())
    }

    fn compute_lp_amount_to_send(
        &self,
        redeem_token_nonce: u64,
        redeem_token_amount: BigUint,
    ) -> BigUint {
        let total_lp_tokens = self.total_lp_tokens_received().get();

        match redeem_token_nonce {
            LAUNCHED_TOKEN_REDEEM_NONCE => {
                let launched_token_final_amount = self.launched_token_final_amount().get();
                redeem_token_amount * total_lp_tokens / launched_token_final_amount / 2u32
            }
            ACCEPTED_TOKEN_REDEEM_NONCE => {
                let accepted_token_final_amount = self.accepted_token_final_amount().get();
                redeem_token_amount * total_lp_tokens / accepted_token_final_amount / 2u32
            }
            _ => BigUint::zero(),
        }
    }
}
