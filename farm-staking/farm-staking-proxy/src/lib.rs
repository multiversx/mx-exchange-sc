#![no_std]

multiversx_sc::imports!();

pub mod dual_yield_token;
pub mod external_contracts_interactions;
pub mod lp_farm_token;
pub mod proxy_actions;
pub mod result_types;

#[multiversx_sc::contract]
pub trait FarmStakingProxy:
    dual_yield_token::DualYieldTokenModule
    + external_contracts_interactions::ExternalContractsInteractionsModule
    + lp_farm_token::LpFarmTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
    + token_send::TokenSendModule
    + sc_whitelist_module::SCWhitelistModule
    + proxy_actions::stake::ProxyStakeModule
    + proxy_actions::claim::ProxyClaimModule
    + proxy_actions::unstake::ProxyUnstakeModule
    + proxy_actions::merge_pos::ProxyMergePosModule
{
    #[init]
    fn init(
        &self,
        lp_farm_address: ManagedAddress,
        staking_farm_address: ManagedAddress,
        pair_address: ManagedAddress,
        staking_token_id: TokenIdentifier,
        lp_farm_token_id: TokenIdentifier,
        staking_farm_token_id: TokenIdentifier,
        lp_token_id: TokenIdentifier,
    ) {
        self.require_sc_address(&lp_farm_address);
        self.require_sc_address(&staking_farm_address);
        self.require_sc_address(&pair_address);

        self.require_valid_token_id(&staking_token_id);
        self.require_valid_token_id(&lp_farm_token_id);
        self.require_valid_token_id(&staking_farm_token_id);
        self.require_valid_token_id(&lp_token_id);

        self.lp_farm_address().set_if_empty(&lp_farm_address);
        self.staking_farm_address()
            .set_if_empty(&staking_farm_address);
        self.pair_address().set_if_empty(&pair_address);

        self.staking_token_id().set_if_empty(&staking_token_id);
        self.lp_farm_token_id().set_if_empty(&lp_farm_token_id);
        self.staking_farm_token_id()
            .set_if_empty(&staking_farm_token_id);
        self.lp_token_id().set_if_empty(&lp_token_id);
    }
}
