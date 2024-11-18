#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod config;
pub mod events;
pub mod pair_actions;
pub mod state;
pub mod temp_owner;
pub mod views;

use pair::read_pair_storage;
use state::{ACTIVE, INACTIVE};

const DEFAULT_TEMPORARY_OWNER_PERIOD_BLOCKS: Blocks = 50;

pub type Blocks = u64;

#[multiversx_sc::contract]
pub trait Router:
    config::ConfigModule
    + read_pair_storage::ReadPairStorageModule
    + events::EventsModule
    + token_send::TokenSendModule
    + pair_actions::enable_swap_by_user::EnableSwapByUserModule
    + pair_actions::multi_pair_swap::MultiPairSwap
    + pair_actions::create::CreateModule
    + pair_actions::upgrade::UpgradeModule
    + pair_actions::tokens::TokensModule
    + pair_actions::fees::FeesModule
    + pair_actions::remove::RemoveModule
    + state::StateModule
    + temp_owner::TempOwnerModule
    + views::ViewsModule
{
    #[init]
    fn init(&self, pair_template_address_opt: OptionalValue<ManagedAddress>) {
        self.state().set(ACTIVE);
        self.pair_creation_enabled().set(false);

        self.temporary_owner_period()
            .set(DEFAULT_TEMPORARY_OWNER_PERIOD_BLOCKS);

        if let OptionalValue::Some(addr) = pair_template_address_opt {
            self.pair_template_address().set(&addr);
        }

        self.owner().set(&self.blockchain().get_caller());
    }

    #[upgrade]
    fn upgrade(&self) {
        self.state().set(INACTIVE);
    }
}
