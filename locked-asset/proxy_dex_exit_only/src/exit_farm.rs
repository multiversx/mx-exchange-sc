use common_structs::{RawResultWrapper, RawResultsType};
use proxy_dex::wrapped_farm_attributes::WrappedFarmTokenAttributes;

elrond_wasm::imports!();

const EXIT_FARM_RESULTS_LEN: usize = 2;

pub struct ExitFarmResultWrapper<M: ManagedTypeApi> {
    pub farming_tokens: EsdtTokenPayment<M>,
    pub reward_tokens: EsdtTokenPayment<M>,
}

mod old_farm_proxy {
    elrond_wasm::imports!();

    use common_structs::RawResultsType;

    #[elrond_wasm::proxy]
    pub trait OldFarmProxy {
        #[payable("*")]
        #[endpoint(exitFarm)]
        fn exit_farm(&self) -> RawResultsType<Self::Api>;
    }
}

#[elrond_wasm::module]
pub trait ExitFarmModule:
    proxy_dex::proxy_common::ProxyCommonModule
    + proxy_dex::events::EventsModule
    + proxy_dex::sc_whitelist::ScWhitelistModule
    + token_send::TokenSendModule
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(exitFarmProxy)]
    fn exit_farm_proxy(
        &self,
        farm_address: ManagedAddress,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment> {
        self.require_is_intermediated_farm(&farm_address);

        let wrapped_farm_token_mapper = self.wrapped_farm_token();
        let payment = self.call_value().single_esdt();
        wrapped_farm_token_mapper.require_same_token(&payment.token_identifier);

        let wrapped_farm_attributes: WrappedFarmTokenAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(&payment, &wrapped_farm_token_mapper);
        let exit_result = self.call_exit_farm(
            farm_address.clone(),
            wrapped_farm_attributes.farm_token.clone(),
        );

        wrapped_farm_token_mapper.nft_burn(payment.token_nonce, &payment.amount);
        self.burn_if_base_asset(&exit_result.farming_tokens);

        let initial_proxy_farming_tokens = wrapped_farm_attributes.proxy_farming_token.clone();
        let caller = self.blockchain().get_caller();
        self.send_payment_non_zero(&caller, &initial_proxy_farming_tokens);
        self.send_payment_non_zero(&caller, &exit_result.reward_tokens);

        self.emit_exit_farm_proxy_event(
            &caller,
            &farm_address,
            payment,
            wrapped_farm_attributes,
            exit_result.reward_tokens.clone(),
        );

        (initial_proxy_farming_tokens, exit_result.reward_tokens).into()
    }

    fn call_exit_farm(
        &self,
        farm_address: ManagedAddress,
        farm_token: EsdtTokenPayment,
    ) -> ExitFarmResultWrapper<Self::Api> {
        let raw_results: RawResultsType<Self::Api> = self
            .old_farm_proxy_obj(farm_address)
            .exit_farm()
            .add_esdt_token_transfer(
                farm_token.token_identifier,
                farm_token.token_nonce,
                farm_token.amount,
            )
            .execute_on_dest_context();

        let mut results_wrapper = RawResultWrapper::new(raw_results);
        results_wrapper.trim_results_front(EXIT_FARM_RESULTS_LEN);

        let farming_tokens = results_wrapper.decode_next_result();
        let reward_tokens = results_wrapper.decode_next_result();

        ExitFarmResultWrapper {
            farming_tokens,
            reward_tokens,
        }
    }

    #[proxy]
    fn old_farm_proxy_obj(&self, sc_address: ManagedAddress) -> old_farm_proxy::Proxy<Self::Api>;
}
