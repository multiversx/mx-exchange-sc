use pair::fee::ProxyTrait as _;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait FeesModule:
    crate::config::ConfigModule
    + pair::read_pair_storage::ReadPairStorageModule
    + crate::temp_owner::TempOwnerModule
    + crate::state::StateModule
{
    #[only_owner]
    #[endpoint(setFeeOn)]
    fn set_fee_on(
        &self,
        pair_address: ManagedAddress,
        fee_to_address: ManagedAddress,
        fee_token: TokenIdentifier,
    ) {
        self.require_active();
        self.check_is_pair_sc(&pair_address);

        let _: IgnoreValue = self
            .pair_contract_proxy_fees(pair_address)
            .set_fee_on(true, fee_to_address, fee_token)
            .execute_on_dest_context();
    }

    #[only_owner]
    #[endpoint(setFeeOff)]
    fn set_fee_off(
        &self,
        pair_address: ManagedAddress,
        fee_to_address: ManagedAddress,
        fee_token: TokenIdentifier,
    ) {
        self.require_active();
        self.check_is_pair_sc(&pair_address);

        let _: IgnoreValue = self
            .pair_contract_proxy_fees(pair_address)
            .set_fee_on(false, fee_to_address, fee_token)
            .execute_on_dest_context();
    }

    #[proxy]
    fn pair_contract_proxy_fees(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;
}
