#![no_std]

elrond_wasm::imports!();

pub mod cancel_unstake;
pub mod fees_accumulation;
pub mod fees_merging;
pub mod tokens_per_user;
pub mod unbond_tokens;

use crate::tokens_per_user::UnstakePair;

#[elrond_wasm::contract]
pub trait TokenUnstakeModule:
    tokens_per_user::TokensPerUserModule
    + unbond_tokens::UnbondTokensModule
    + cancel_unstake::CancelUnstakeModule
    + fees_accumulation::FeesAccumulationModule
    + fees_merging::FeesMergingModule
    + utils::UtilsModule
    + energy_query::EnergyQueryModule
    + energy_factory::penalty::LocalPenaltyModule
    + energy_factory::lock_options::LockOptionsModule
{
    /// Needs burn role for both the unlocked and locked token
    #[init]
    fn init(&self, unbond_epochs: u64, energy_factory_address: ManagedAddress) {
        self.require_sc_address(&energy_factory_address);

        self.unbond_epochs().set(unbond_epochs);
        self.energy_factory_address().set(&energy_factory_address);
    }

    #[payable("*")]
    #[endpoint(depositUserTokens)]
    fn deposit_user_tokens(&self, user: ManagedAddress) {
        let caller = self.blockchain().get_caller();
        let energy_factory_address = self.energy_factory_address().get();
        require!(
            caller == energy_factory_address,
            "Only energy factory SC can call this endpoint"
        );

        let [locked_tokens, unlocked_tokens] = self.call_value().multi_esdt();
        let current_epoch = self.blockchain().get_block_epoch();
        let unbond_epochs = self.unbond_epochs().get();
        let unlock_epoch = current_epoch + unbond_epochs;
        self.unlocked_tokens_for_user(&user)
            .update(|unstake_pairs| {
                let unstake_pair = UnstakePair {
                    unlock_epoch,
                    locked_tokens,
                    unlocked_tokens,
                };
                unstake_pairs.push(unstake_pair);
            });
    }
}
