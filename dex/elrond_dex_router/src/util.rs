elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm_derive::module]
pub trait UtilModule {
    #[inline]
    fn is_active(&self) -> bool {
        self.state().get()
    }

    #[endpoint(setPairCreationEnabled)]
    fn set_pair_creation_enabled(&self, enabled: bool) -> SCResult<()> {
        self.require_owner()?;
        self.pair_creation_enabled().set(&enabled);
        Ok(())
    }

    fn require_owner(&self) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let owner = self.owner().get();
        require!(caller == owner, "Permission denied");
        Ok(())
    }

    fn send_tokens(
        &self,
        token: &TokenIdentifier,
        amount: &Self::BigUint,
        destination: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        if amount > &0 {
            let (function, gas_limit) = match opt_accept_funds_func {
                OptionalArg::Some(accept_funds_func) => (
                    accept_funds_func.as_slice(),
                    self.transfer_exec_gas_limit().get(),
                ),
                OptionalArg::None => {
                    let no_func: &[u8] = &[];
                    (no_func, 0u64)
                }
            };

            let _ = self.send().direct_esdt_execute(
                destination,
                token,
                amount,
                gas_limit,
                function,
                &ArgBuffer::new(),
            )?;
        }

        Ok(())
    }

    #[endpoint(setTransferExecGasLimit)]
    fn set_transfer_exec_gas_limit(&self, gas_limit: u64) -> SCResult<()> {
        self.require_owner()?;
        self.transfer_exec_gas_limit().set(&gas_limit);
        Ok(())
    }

    #[view(getTranferExecGasLimit)]
    #[storage_mapper("transfer_exec_gas_limit")]
    fn transfer_exec_gas_limit(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[view(getOwner)]
    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<Self::Storage, bool>;

    #[view(getPairCreationEnabled)]
    #[storage_mapper("pair_creation_enabled")]
    fn pair_creation_enabled(&self) -> SingleValueMapper<Self::Storage, bool>;

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<Self::Storage, BoxedBytes>;
}
