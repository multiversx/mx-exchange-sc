use simple_lock::{farm_interactions::ExitFarmResult, proxy_farm::ExitFarmThroughProxyResultType};

elrond_wasm::imports!();

pub struct OldFarmExitResults<M: ManagedTypeApi> {
    pub initial_farming_tokens: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
}

impl<M: ManagedTypeApi> ExitFarmResult<M> for OldFarmExitResults<M> {
    fn new(
        initial_farming_tokens: EsdtTokenPayment<M>,
        reward_tokens: EsdtTokenPayment<M>,
        _additional_tokens: ManagedVec<M, EsdtTokenPayment<M>>,
    ) -> Self {
        OldFarmExitResults {
            initial_farming_tokens,
            reward_tokens,
        }
    }

    #[inline]
    fn get_initial_farming_tokens(&self) -> EsdtTokenPayment<M> {
        self.initial_farming_tokens.clone()
    }

    #[inline]
    fn get_reward_tokens(&self) -> EsdtTokenPayment<M> {
        self.reward_tokens.clone()
    }

    #[inline]
    fn get_additional_results_expected_len() -> usize {
        0
    }

    #[inline]
    fn get_additional_tokens(&self) -> ManagedVec<M, EsdtTokenPayment<M>> {
        ManagedVec::new()
    }
}

#[elrond_wasm::module]
pub trait OldFarmsExitModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::proxy_lp::ProxyLpModule
    + simple_lock::proxy_farm::ProxyFarmModule
    + simple_lock::lp_interactions::LpInteractionsModule
    + simple_lock::farm_interactions::FarmInteractionsModule
    + simple_lock::token_attributes::TokenAttributesModule
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(exitFarmOldToken)]
    fn exit_farm_old_token(&self) -> ExitFarmThroughProxyResultType<Self::Api> {
        self.exit_farm_base_impl::<OldFarmExitResults<Self::Api>>(OptionalValue::None)
    }
}
