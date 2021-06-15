elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::factory;
use super::util;

#[elrond_wasm_derive::module]
pub trait PairManagerModule: util::UtilModule + factory::FactoryModule {
    #[endpoint(setFeeOn)]
    fn set_fee_on(
        &self,
        pair_address: Address,
        fee_to_address: Address,
        fee_token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        self.check_is_pair_sc(&pair_address)?;

        self.pair_contract_proxy(pair_address)
            .setFeeOn(true, fee_to_address, fee_token)
            .execute_on_dest_context();

        Ok(())
    }

    #[endpoint(setFeeOff)]
    fn set_fee_off(
        &self,
        pair_address: Address,
        fee_to_address: Address,
        fee_token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        self.check_is_pair_sc(&pair_address)?;

        self.pair_contract_proxy(pair_address)
            .setFeeOn(false, fee_to_address, fee_token)
            .execute_on_dest_context();

        Ok(())
    }

    fn pause_pair(&self, address: Address) {
        self.pair_contract_proxy(address)
            .pause()
            .execute_on_dest_context();
    }

    fn resume_pair(&self, address: Address) {
        self.pair_contract_proxy(address)
            .resume()
            .execute_on_dest_context();
    }

    fn get_lp_token_for_pair(&self, address: &Address) -> TokenIdentifier {
        self.pair_contract_proxy(address.clone())
            .getLpTokenIdentifier()
            .execute_on_dest_context()
    }

    fn set_lp_token_for_pair(&self, address: &Address, token_id: &TokenIdentifier) {
        self.pair_contract_proxy(address.clone())
            .setLpTokenIdentifier(token_id.clone())
            .execute_on_dest_context();
    }

    #[proxy]
    fn pair_contract_proxy(&self, to: Address) -> elrond_dex_pair::Proxy<Self::SendApi>;
}
