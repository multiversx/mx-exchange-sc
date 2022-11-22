#![no_std]

elrond_wasm::imports!();

pub mod exit_farm;
pub mod exit_pair;

#[elrond_wasm::contract]
pub trait ProxyDexExitOnly:
    proxy_dex::proxy_common::ProxyCommonModule
    + proxy_dex::sc_whitelist::ScWhitelistModule
    + proxy_dex::events::EventsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
    + token_send::TokenSendModule
    + exit_farm::ExitFarmModule
    + exit_pair::ExitPairModule
{
    #[init]
    fn init(&self) {}
}
