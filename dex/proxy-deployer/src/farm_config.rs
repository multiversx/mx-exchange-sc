elrond_wasm::imports!();

use config::ProxyTrait as _;
use farm_token::ProxyTrait as _;
use pausable::ProxyTrait as _;

const GAS_LIMIT_FOR_CLEANUP: u64 = 1_000_000;

#[elrond_wasm::module]
pub trait FarmConfigModule {
    #[only_owner]
    #[endpoint(pauseFarm)]
    fn pause_farm(&self, farm_address: ManagedAddress) {
        self.farm_config_proxy(farm_address)
            .pause()
            .execute_on_dest_context_ignore_result();
    }

    #[only_owner]
    #[endpoint(resumeFarm)]
    fn resume_farm(&self, farm_address: ManagedAddress) {
        self.farm_config_proxy(farm_address)
            .resume()
            .execute_on_dest_context_ignore_result();
    }

    #[only_owner]
    #[endpoint(setFarmBurnGasLimit)]
    fn set_farm_burn_gas_limit(&self, farm_address: ManagedAddress, gas_limit: u64) {
        self.farm_config_proxy(farm_address)
            .set_burn_gas_limit(gas_limit)
            .execute_on_dest_context_ignore_result();
    }

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerFarmToken)]
    fn register_farm_token(
        &self,
        farm_address: ManagedAddress,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        let payment = self.call_value().egld_value();
        let gas_left = self.blockchain().get_gas_left();
        let gas_for_call = gas_left - GAS_LIMIT_FOR_CLEANUP;

        self.farm_config_proxy(farm_address)
            .register_farm_token(token_display_name, token_ticker, num_decimals)
            .with_egld_transfer(payment)
            .with_gas_limit(gas_for_call)
            .execute_on_dest_context_ignore_result();
    }

    #[proxy]
    fn farm_config_proxy(&self, sc_address: ManagedAddress) -> farm::Proxy<Self::Api>;
}
