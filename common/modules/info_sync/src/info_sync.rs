#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use core::iter::FromIterator;

const ACCEPT_INFO_ENDPOINT_NAME: &[u8] = b"acceptInformation";
const ACTION_CALLBACK_NAME: &[u8] = b"takeActionOnInformationReceive";

#[elrond_wasm::module]
pub trait InfoSyncModule {
    #[only_owner]
    #[endpoint(addClone)]
    fn add_clone(&self, clone_address: Address) {
        self.clones().insert(clone_address);
    }

    fn broadcast_information(&self, info: BoxedBytes) -> SCResult<()> {
        let big_zero = Self::BigUint::zero();
        let gas_left = self.blockchain().get_gas_left();
        let clones_len = self.clones().len() as u64;
        let per_clone_gas = gas_left / (clones_len + 1);
        let endpoint = BoxedBytes::from(ACCEPT_INFO_ENDPOINT_NAME);
        let mut arg_buffer = ArgBuffer::new();
        arg_buffer.push_argument_bytes(info.as_slice());

        for clone in self.clones().iter() {
            self.send().direct_egld_execute(
                &clone,
                &big_zero,
                per_clone_gas,
                endpoint.as_slice(),
                &arg_buffer,
            )?;
        }

        Ok(())
    }

    #[endpoint(acceptInformation)]
    fn accept_information(&self, info: BoxedBytes) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        require!(self.clones().contains(&caller), "Not a clone");
        self.received_info().insert(caller, info);

        if self.received_info().len() == self.clones().len() {
            self.take_action();
        }

        Ok(())
    }

    fn take_action(&self) {
        self.send().execute_on_dest_context_raw(
            self.blockchain().get_gas_left(),
            &self.blockchain().get_sc_address(),
            &Self::BigUint::zero(),
            BoxedBytes::from(ACTION_CALLBACK_NAME).as_slice(),
            &ArgBuffer::new(),
        );
        self.received_info().clear();
    }

    #[view(getReceivedInfo)]
    fn get_received_info(&self) -> MultiResultVec<(Address, BoxedBytes)> {
        MultiResultVec::from_iter(
            self.received_info()
                .iter()
                .collect::<Vec<(Address, BoxedBytes)>>(),
        )
    }

    #[storage_mapper("InfoSync:received_info")]
    fn received_info(&self) -> SafeMapMapper<Self::Storage, Address, BoxedBytes>;

    #[view(getClones)]
    #[storage_mapper("InfoSync:clones")]
    fn clones(&self) -> SafeSetMapper<Self::Storage, Address>;
}
