#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod config;
pub mod events;
pub mod pair_actions;
pub mod state;
pub mod temp_owner;
pub mod views;

use config::DISABLED;
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
    + pair_actions::enable_buyback_and_burn::EnableBuybackAndBurnModule
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
    fn init(
        &self,
        token_to_buy: TokenIdentifier,
        pair_template_address_opt: OptionalValue<ManagedAddress>,
    ) {
        self.set_token_to_buy(token_to_buy);

        self.state().set(ACTIVE);
        self.pair_creation_enabled().set(DISABLED);
        self.temporary_owner_period()
            .set(DEFAULT_TEMPORARY_OWNER_PERIOD_BLOCKS);

        if let OptionalValue::Some(addr) = pair_template_address_opt {
            self.pair_template_address().set(&addr);
        }

        let caller = self.blockchain().get_caller();
        self.owner().set(caller);
    }

    #[upgrade]
    fn upgrade(&self) {
        self.state().set(INACTIVE);
    }
}
