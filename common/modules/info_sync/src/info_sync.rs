#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const ACCEPT_INFO_ENDPOINT_NAME: &[u8] = b"acceptInformation";

// #[endpoint(takeActionOnInformationReceive)]
// fn take_action_on_information_receive(
//     &self,
//     #[var_args] args: MultiArgVec<MultiArg2<Address, BoxedBytes>>,
// )
const ACTION_CALLBACK_NAME: &[u8] = b"takeActionOnInformationReceive";

#[elrond_wasm::module]
pub trait InfoSyncModule {
    #[only_owner]
    #[endpoint(addClone)]
    fn add_clone(&self, clone_address: Address) -> SCResult<()> {
        require!(
            !self.clones().contains(&clone_address),
            "Adress already added"
        );

        let my_address = self.blockchain().get_sc_address();
        let my_shard = self.blockchain().get_shard_of_address(&my_address);
        let clone_shard = self.blockchain().get_shard_of_address(&clone_address);

        require!(my_shard != clone_shard, "Same shard as own shard");
        for element in self.clones().iter() {
            let element_shard = self.blockchain().get_shard_of_address(&element);
            require!(
                element_shard != clone_shard,
                "Same shard as another clone address"
            );
        }

        self.clones().insert(clone_address);
        Ok(())
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
                &endpoint.as_slice(),
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
            let collected_info = self.collect_info();
            self.received_info().clear();
            self.take_action(collected_info);
        }

        Ok(())
    }

    fn collect_info(&self) -> ArgBuffer {
        let mut collected_info = ArgBuffer::new();
        for elem in self.received_info().iter() {
            collected_info.push_argument_bytes(elem.0.as_bytes());
            collected_info.push_argument_bytes(elem.1.as_slice());
        }
        collected_info
    }

    fn take_action(&self, collected_info: ArgBuffer) {
        self.send().execute_on_dest_context_raw(
            self.blockchain().get_gas_left(),
            &self.blockchain().get_sc_address(),
            &Self::BigUint::zero(),
            &BoxedBytes::from(ACTION_CALLBACK_NAME).as_slice(),
            &collected_info,
        );
    }

    #[storage_mapper("InfoSync:received_info")]
    fn received_info(&self) -> SafeMapMapper<Self::Storage, Address, BoxedBytes>;

    #[storage_mapper("InfoSync:clones")]
    fn clones(&self) -> SafeSetMapper<Self::Storage, Address>;
}
