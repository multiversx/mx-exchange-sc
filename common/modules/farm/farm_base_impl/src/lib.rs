#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]
#![feature(trait_alias)]
#![feature(associated_type_defaults)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod base_farm_init;
pub mod base_farm_validation;
pub mod base_traits_impl;
pub mod claim_rewards;
pub mod compound_rewards;
pub mod enter_farm;
pub mod exit_farm;

#[multiversx_sc::module]
pub trait FarmBaseImpl:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + events::EventsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + base_farm_init::BaseFarmInitModule
    + base_farm_validation::BaseFarmValidationModule
    + enter_farm::BaseEnterFarmModule
    + claim_rewards::BaseClaimRewardsModule
    + compound_rewards::BaseCompoundRewardsModule
    + exit_farm::BaseExitFarmModule
    + utils::UtilsModule
{
}
