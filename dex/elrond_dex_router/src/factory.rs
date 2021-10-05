elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use core::iter::FromIterator;
const TEMPORARY_OWNER_PERIOD_BLOCKS: u64 = 50;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi)]
pub struct PairTokens<M: ManagedTypeApi> {
    pub first_token_id: TokenIdentifier<M>,
    pub second_token_id: TokenIdentifier<M>,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct PairContractMetadata<M: ManagedTypeApi> {
    first_token_id: TokenIdentifier<M>,
    second_token_id: TokenIdentifier<M>,
    address: ManagedAddress<M>,
}

#[elrond_wasm::module]
pub trait FactoryModule {
    fn init_factory(&self) {
        self.pair_code_ready().set_if_empty(&false);
        self.pair_code().set_if_empty(&ManagedBuffer::new());
        self.temporary_owner_period()
            .set_if_empty(&TEMPORARY_OWNER_PERIOD_BLOCKS);
    }

    fn create_pair(
        &self,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
        owner: &ManagedAddress,
        total_fee_percent: u64,
        special_fee_percent: u64,
    ) -> SCResult<ManagedAddress> {
        require!(self.pair_code_ready().get(), "Pair code not ready");
        let code_metadata = CodeMetadata::UPGRADEABLE;
        let gas_left = self.blockchain().get_gas_left();
        let amount = self.types().big_uint_zero();

        let mut arg_buffer = ManagedArgBuffer::new_empty(self.type_manager());
        let code = self.pair_code().get();
        arg_buffer.push_arg(first_token_id);
        arg_buffer.push_arg(second_token_id);
        arg_buffer.push_arg(self.blockchain().get_sc_address());
        arg_buffer.push_arg(owner);
        arg_buffer.push_arg(&total_fee_percent.to_be_bytes()[..]);
        arg_buffer.push_arg(&special_fee_percent.to_be_bytes()[..]);

        let (new_address, _) =
            self.raw_vm_api()
                .deploy_contract(gas_left, &amount, &code, code_metadata, &arg_buffer);

        self.pair_map().insert(
            PairTokens {
                first_token_id: first_token_id.clone(),
                second_token_id: second_token_id.clone(),
            },
            new_address.clone(),
        );
        self.pair_temporary_owner().insert(
            new_address.clone(),
            (
                self.blockchain().get_caller(),
                self.blockchain().get_block_nonce(),
            ),
        );
        Ok(new_address)
    }

    fn upgrade_pair(
        &self,
        pair_address: &ManagedAddress,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
        owner: &ManagedAddress,
        total_fee_percent: u64,
        special_fee_percent: u64,
    ) -> SCResult<()> {
        require!(self.pair_code_ready().get(), "Pair code not ready");

        let mut arg_buffer = ManagedArgBuffer::new_empty(self.type_manager());
        arg_buffer.push_arg(first_token_id);
        arg_buffer.push_arg(second_token_id);
        arg_buffer.push_arg(self.blockchain().get_sc_address());
        arg_buffer.push_arg(owner);
        arg_buffer.push_arg(&total_fee_percent.to_be_bytes()[..]);
        arg_buffer.push_arg(&special_fee_percent.to_be_bytes()[..]);

        self.raw_vm_api().upgrade_contract(
            pair_address,
            self.blockchain().get_gas_left(),
            &self.types().big_uint_zero(),
            &self.pair_code().get(),
            CodeMetadata::UPGRADEABLE,
            &arg_buffer,
        );
        Ok(())
    }

    fn start_pair_construct(&self) {
        self.pair_code_ready().set(&false);
        self.pair_code().set(&ManagedBuffer::new());
    }

    fn end_pair_construct(&self) {
        self.pair_code_ready().set(&true);
    }

    fn append_pair_code(&self, part: &ManagedBuffer) -> SCResult<()> {
        require!(
            !self.pair_code_ready().get(),
            "Pair construction not started"
        );

        let mut existent = self.pair_code().get();
        existent.append(part);

        self.pair_code().set(&existent);
        Ok(())
    }

    #[storage_mapper("pair_map")]
    fn pair_map(&self) -> MapMapper<PairTokens<Self::Api>, ManagedAddress>;

    #[view(getAllPairsManagedAddresses)]
    fn get_all_pairs_addresses(&self) -> MultiResultVec<ManagedAddress> {
        self.pair_map().values().collect()
    }

    #[view(getAllPairTokens)]
    fn get_all_token_pairs(&self) -> MultiResultVec<PairTokens<Self::Api>> {
        self.pair_map().keys().collect()
    }

    #[view(getAllPairContractMetadata)]
    fn get_all_pair_contract_metadata(&self) -> MultiResultVec<PairContractMetadata<Self::Api>> {
        let map: Vec<PairContractMetadata<Self::Api>> = self
            .pair_map()
            .iter()
            .map(|x| PairContractMetadata {
                first_token_id: x.0.first_token_id,
                second_token_id: x.0.second_token_id,
                address: x.1,
            })
            .collect();
        MultiResultVec::from_iter(map)
    }

    fn get_pair_temporary_owner(&self, pair_address: &ManagedAddress) -> Option<ManagedAddress> {
        let result = self.pair_temporary_owner().get(pair_address);

        match result {
            Some((temporary_owner, creation_block)) => {
                let expire_block = creation_block + self.temporary_owner_period().get();

                if expire_block >= self.blockchain().get_block_nonce() {
                    self.pair_temporary_owner().remove(pair_address);
                    None
                } else {
                    Some(temporary_owner)
                }
            }
            None => None,
        }
    }

    #[endpoint(clearPairTemporaryOwnerStorage)]
    fn clear_pair_temporary_owner_storage(&self) -> SCResult<usize> {
        only_owner!(self, "No permissions");
        let size = self.pair_temporary_owner().len();
        self.pair_temporary_owner().clear();
        Ok(size)
    }

    #[endpoint(setTemporaryOwnerPeriod)]
    fn set_temporary_owner_period(&self, period_blocks: u64) -> SCResult<()> {
        only_owner!(self, "No permissions");
        self.temporary_owner_period().set(&period_blocks);
        Ok(())
    }

    #[view(getPairCode)]
    #[storage_mapper("pair_code")]
    fn pair_code(&self) -> SingleValueMapper<ManagedBuffer>;

    #[view(getPairCodeReady)]
    #[storage_mapper("pair_code_ready")]
    fn pair_code_ready(&self) -> SingleValueMapper<bool>;

    #[view(getTemporaryOwnerPeriod)]
    #[storage_mapper("temporary_owner_period")]
    fn temporary_owner_period(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("pair_temporary_owner")]
    fn pair_temporary_owner(&self) -> MapMapper<ManagedAddress, (ManagedAddress, u64)>;
}
