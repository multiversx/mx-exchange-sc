#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

pub mod config;
mod events;
mod farm_token;
mod rewards;

use common_structs::{FarmTokenAttributes, Nonce};
use config::State;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type ExitFarmResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[elrond_wasm::contract]
pub trait Farm:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
    + farm_token::FarmTokenModule
    + events::EventsModule
{
    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_nonce] token_nonce: Nonce,
        #[payment_amount] amount: BigUint,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<ExitFarmResultType<Self::Api>> {
        let state = self.state().get();
        require!(state == State::Migrate, "Must be in migrate state to exit");

        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Bad input token");

        let farm_attributes = self.get_farm_attributes(&payment_token_id, token_nonce)?;

        let farming_token_id = self.farming_token_id().get();
        let initial_farming_token_amount = self.rule_of_three_non_zero_result(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.initial_farming_amount,
        )?;

        let mut reward_token_id = self.reward_token_id().get();
        let mut compounded_rewards = self.rule_of_three(
            &amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        );

        let caller = self.blockchain().get_caller();
        self.burn_farm_tokens(&payment_token_id, token_nonce, &amount);
        self.send_back_farming_tokens(
            &farming_token_id,
            &initial_farming_token_amount,
            &caller,
            &opt_accept_funds_func,
        )?;

        let mut reward_nonce = 0u64;
        if compounded_rewards > 0u64 {
            self.send_rewards(
                &mut reward_token_id,
                &mut reward_nonce,
                &mut compounded_rewards,
                &caller,
                farm_attributes.with_locked_rewards,
                farm_attributes.original_entering_epoch,
                &opt_accept_funds_func,
            )?;
        }

        self.emit_exit_farm_event(
            &caller,
            &farming_token_id,
            &initial_farming_token_amount,
            &self.farming_token_reserve().get(),
            &farm_token_id,
            token_nonce,
            &amount,
            &self.get_farm_token_supply(),
            &reward_token_id,
            reward_nonce,
            &compounded_rewards,
            &self.reward_reserve().get(),
            &farm_attributes,
        );
        Ok(MultiResult2::from((
            self.create_payment(&farming_token_id, 0, &initial_farming_token_amount),
            self.create_payment(&reward_token_id, reward_nonce, &compounded_rewards),
        )))
    }

    fn send_back_farming_tokens(
        &self,
        farming_token_id: &TokenIdentifier,
        farming_amount: &BigUint,
        destination: &ManagedAddress,
        opt_accept_funds_func: &OptionalArg<ManagedBuffer>,
    ) -> SCResult<()> {
        self.decrease_farming_token_reserve(farming_amount)?;
        self.transfer_execute_custom(
            destination,
            farming_token_id,
            0,
            farming_amount,
            opt_accept_funds_func,
        )?;
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
        _amount: BigUint,
        _attributes_raw: ManagedBuffer,
    ) -> SCResult<BigUint> {
        Ok(BigUint::zero())
    }

    fn decrease_farming_token_reserve(&self, amount: &BigUint) -> SCResult<()> {
        let current = self.farming_token_reserve().get();
        require!(&current >= amount, "Not enough farming reserve");
        self.farming_token_reserve().set(&(&current - amount));
        Ok(())
    }
}
