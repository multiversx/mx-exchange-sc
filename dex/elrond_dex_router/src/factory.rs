elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use core::iter::FromIterator;
const TEMPORARY_OWNER_PERIOD_BLOCKS: u64 = 50;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi)]
pub struct PairTokens {
    pub first_token_id: TokenIdentifier,
    pub second_token_id: TokenIdentifier,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct PairContractMetadata {
    first_token_id: TokenIdentifier,
    second_token_id: TokenIdentifier,
    address: Address,
}

#[elrond_wasm_derive::module]
pub trait FactoryModule {
    fn init_factory(&self) {
        self.pair_code_ready().set_if_empty(&false);
        self.pair_code().set_if_empty(&BoxedBytes::empty());
        self.temporary_owner_period()
            .set_if_empty(&TEMPORARY_OWNER_PERIOD_BLOCKS);
    }

    fn create_pair(
        &self,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
        owner: &Address,
        total_fee_percent: u64,
        special_fee_percent: u64,
    ) -> SCResult<Address> {
        require!(self.pair_code_ready().get(), "Pair code not ready");
        let code_metadata = CodeMetadata::UPGRADEABLE;
        let gas_left = self.blockchain().get_gas_left();
        let amount = 0u64.into();
        let mut arg_buffer = ArgBuffer::new();
        let code = self.pair_code().get();
        arg_buffer.push_argument_bytes(first_token_id.as_esdt_identifier());
        arg_buffer.push_argument_bytes(second_token_id.as_esdt_identifier());
        arg_buffer.push_argument_bytes(self.blockchain().get_sc_address().as_bytes());
        arg_buffer.push_argument_bytes(owner.as_bytes());
        arg_buffer.push_argument_bytes(&total_fee_percent.to_be_bytes()[..]);
        arg_buffer.push_argument_bytes(&special_fee_percent.to_be_bytes()[..]);
        let new_address =
            self.send()
                .deploy_contract(gas_left, &amount, &code, code_metadata, &arg_buffer);
        require!(new_address.is_some(), "deploy failed");
        self.pair_map().insert(
            PairTokens {
                first_token_id: first_token_id.clone(),
                second_token_id: second_token_id.clone(),
            },
            new_address.clone().unwrap().clone(),
        );
        self.pair_temporary_owner().insert(
            new_address.clone().unwrap().clone(),
            (
                self.blockchain().get_caller(),
                self.blockchain().get_block_nonce(),
            ),
        );
        Ok(new_address.unwrap())
    }

    fn upgrade_pair(
        &self,
        pair_address: &Address,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
        owner: &Address,
        total_fee_percent: u64,
        special_fee_percent: u64,
    ) -> SCResult<()> {
        require!(self.pair_code_ready().get(), "Pair code not ready");

        let mut arg_buffer = ArgBuffer::new();
        arg_buffer.push_argument_bytes(first_token_id.as_esdt_identifier());
        arg_buffer.push_argument_bytes(second_token_id.as_esdt_identifier());
        arg_buffer.push_argument_bytes(self.blockchain().get_sc_address().as_bytes());
        arg_buffer.push_argument_bytes(owner.as_bytes());
        arg_buffer.push_argument_bytes(&total_fee_percent.to_be_bytes()[..]);
        arg_buffer.push_argument_bytes(&special_fee_percent.to_be_bytes()[..]);

        self.send().upgrade_contract(
            pair_address,
            self.blockchain().get_gas_left(),
            &0u64.into(),
            &self.pair_code().get(),
            CodeMetadata::DEFAULT,
            &arg_buffer,
        );
        Ok(())
    }

    fn start_pair_construct(&self) {
        self.pair_code_ready().set(&false);
        self.pair_code().set(&BoxedBytes::empty());
    }

    fn end_pair_construct(&self) {
        self.pair_code_ready().set(&true);
    }

    fn append_pair_code(&self, part: &BoxedBytes) -> SCResult<()> {
        require!(
            !self.pair_code_ready().get(),
            "Pair construction not started"
        );
        let existent = self.pair_code().get();
        let new_code = BoxedBytes::from_concat(&[existent.as_slice(), part.as_slice()]);
        self.pair_code().set(&new_code);
        Ok(())
    }

    #[storage_mapper("pair_map")]
    fn pair_map(&self) -> SafeMapMapper<Self::Storage, PairTokens, Address>;

    #[view(getAllPairsAddresses)]
    fn get_all_pairs_addresses(&self) -> MultiResultVec<Address> {
        self.pair_map().values().collect()
    }

    #[view(getAllPairTokens)]
    fn get_all_token_pairs(&self) -> MultiResultVec<PairTokens> {
        self.pair_map().keys().collect()
    }

    #[view(getAllPairContractMetadata)]
    fn get_all_pair_contract_metadata(&self) -> MultiResultVec<PairContractMetadata> {
        let map: Vec<PairContractMetadata> = self
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

    fn get_pair_temporary_owner(&self, pair_address: &Address) -> Option<Address> {
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
    fn pair_code(&self) -> SingleValueMapper<Self::Storage, BoxedBytes>;

    #[view(getPairCodeReady)]
    #[storage_mapper("pair_code_ready")]
    fn pair_code_ready(&self) -> SingleValueMapper<Self::Storage, bool>;

    #[view(getTemporaryOwnerPeriod)]
    #[storage_mapper("temporary_owner_period")]
    fn temporary_owner_period(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[storage_mapper("pair_temporary_owner")]
    fn pair_temporary_owner(&self) -> SafeMapMapper<Self::Storage, Address, (Address, u64)>;
}
