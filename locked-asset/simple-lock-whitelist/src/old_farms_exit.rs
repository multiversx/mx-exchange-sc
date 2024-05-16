use simple_lock::{
    error_messages::INVALID_PAYMENTS_RECEIVED_FROM_FARM_ERR_MSG,
    farm_interactions::ExitFarmResultWrapper,
    proxy_farm::{ExitFarmThroughProxyResultType, FarmProxyTokenAttributes},
};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct OldFarmExitResults<M: ManagedTypeApi> {
    pub initial_farming_tokens: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
    pub remaining_farm_tokens: EsdtTokenPayment<M>,
}

mod old_farm_proxy {
    use super::OldFarmExitResults;

    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait OldFarmExitResultsrmProxy {
        #[payable("*")]
        #[endpoint(exitFarm)]
        fn exit_farm(&self) -> OldFarmExitResults<Self::Api>;
    }
}

#[multiversx_sc::module]
pub trait OldFarmsExitModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
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
        let payment = self.call_value().single_esdt();

        let farm_proxy_token_attributes: FarmProxyTokenAttributes<Self::Api> =
            self.validate_payment_and_get_farm_proxy_token_attributes(&payment);

        let farm_address = self.try_get_farm_address(
            &farm_proxy_token_attributes.farming_token_id,
            farm_proxy_token_attributes.farm_type,
        );
        let result: OldFarmExitResults<Self::Api> = self
            .old_farm_proxy(farm_address)
            .exit_farm()
            .with_esdt_transfer(EsdtTokenPayment::new(
                payment.token_identifier,
                payment.token_nonce,
                payment.amount,
            ))
            .execute_on_dest_context();

        let exit_farm_result = ExitFarmResultWrapper {
            initial_farming_tokens: result.initial_farming_tokens,
            reward_tokens: result.reward_tokens,
        };
        require!(
            exit_farm_result.initial_farming_tokens.token_identifier
                == farm_proxy_token_attributes.farming_token_id,
            INVALID_PAYMENTS_RECEIVED_FROM_FARM_ERR_MSG
        );

        let lp_proxy_token = self.lp_proxy_token();
        let lp_proxy_token_payment = EsdtTokenPayment::new(
            lp_proxy_token.get_token_id(),
            farm_proxy_token_attributes.farming_token_locked_nonce,
            exit_farm_result.initial_farming_tokens.amount,
        );
        let caller = self.blockchain().get_caller();
        self.send().direct_esdt(
            &caller,
            &lp_proxy_token_payment.token_identifier,
            lp_proxy_token_payment.token_nonce,
            &lp_proxy_token_payment.amount,
        );

        if exit_farm_result.reward_tokens.amount > 0 {
            self.send().direct_esdt(
                &caller,
                &exit_farm_result.reward_tokens.token_identifier,
                exit_farm_result.reward_tokens.token_nonce,
                &exit_farm_result.reward_tokens.amount,
            );
        }

        (lp_proxy_token_payment, exit_farm_result.reward_tokens).into()
    }

    #[proxy]
    fn old_farm_proxy(&self, sc_address: ManagedAddress) -> old_farm_proxy::Proxy<Self::Api>;
}
