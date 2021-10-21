elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub enum State {
    Inactive,
    Active,
    ActiveNoSwaps,
}

#[elrond_wasm::module]
pub trait ConfigModule: token_send::TokenSendModule {
    #[endpoint]
    fn set_transfer_exec_gas_limit(&self, gas_limit: u64) -> SCResult<()> {
        self.require_permissions()?;
        self.transfer_exec_gas_limit().set(&gas_limit);
        Ok(())
    }

    #[endpoint]
    fn set_extern_swap_gas_limit(&self, gas_limit: u64) -> SCResult<()> {
        self.require_permissions()?;
        self.extern_swap_gas_limit().set(&gas_limit);
        Ok(())
    }

    fn require_permissions(&self) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let owner = self.router_owner_address().get();
        let router = self.router_address().get();
        require!(caller == owner || caller == router, "Permission denied");
        Ok(())
    }

    #[endpoint]
    fn pause(&self) -> SCResult<()> {
        self.require_permissions()?;
        self.state().set(&State::Inactive);
        Ok(())
    }

    #[endpoint]
    fn resume(&self) -> SCResult<()> {
        self.require_permissions()?;
        self.state().set(&State::Active);
        Ok(())
    }

    #[endpoint(setStateActiveNoSwaps)]
    fn set_state_active_no_swaps(&self) -> SCResult<()> {
        self.require_permissions()?;
        self.state().set(&State::ActiveNoSwaps);
        Ok(())
    }

    #[view(getLpTokenIdentifier)]
    fn get_lp_token_identifier(&self) -> TokenIdentifier {
        self.lp_token_identifier().get()
    }

    #[endpoint(setFeePercents)]
    fn set_fee_percent(&self, total_fee_percent: u64, special_fee_percent: u64) -> SCResult<()> {
        self.require_permissions()?;
        self.try_set_fee_percents(total_fee_percent, special_fee_percent)
    }

    fn try_set_fee_percents(
        &self,
        total_fee_percent: u64,
        special_fee_percent: u64,
    ) -> SCResult<()> {
        require!(
            total_fee_percent >= special_fee_percent && total_fee_percent < 100_000,
            "Bad percents"
        );
        self.total_fee_percent().set(&total_fee_percent);
        self.special_fee_percent().set(&special_fee_percent);
        Ok(())
    }

    #[view(getTotalFeePercent)]
    #[storage_mapper("total_fee_percent")]
    fn total_fee_percent(&self) -> SingleValueMapper<u64>;

    #[view(getSpecialFee)]
    #[storage_mapper("special_fee_percent")]
    fn special_fee_percent(&self) -> SingleValueMapper<u64>;

    #[view(getRouterManagedAddress)]
    #[storage_mapper("router_address")]
    fn router_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getRouterOwnerManagedAddress)]
    #[storage_mapper("router_owner_address")]
    fn router_owner_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<State>;

    #[view(getExternSwapGasLimit)]
    #[storage_mapper("extern_swap_gas_limit")]
    fn extern_swap_gas_limit(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("lpTokenIdentifier")]
    fn lp_token_identifier(&self) -> SingleValueMapper<TokenIdentifier>;
}
