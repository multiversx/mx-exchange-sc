elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use core::iter::FromIterator;

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

#[elrond_wasm_derive::module(FactoryModuleImpl)]
pub trait FactoryModule {
    fn init(&self) {
        self.pair_code_ready().set(&false);
        self.pair_code().set(&BoxedBytes::empty());
    }

    fn create_pair(
        &self,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
        owner: &Address,
        total_fee_precent: u64,
        special_fee_precent: u64,
    ) -> Address {
        if !self.pair_code_ready().get() {
            return Address::zero();
        }
        let code_metadata = CodeMetadata::UPGRADEABLE;
        let gas_left = self.blockchain().get_gas_left();
        let amount = BigUint::zero();
        let mut arg_buffer = ArgBuffer::new();
        let code = self.pair_code().get();
        arg_buffer.push_argument_bytes(first_token_id.as_esdt_identifier());
        arg_buffer.push_argument_bytes(second_token_id.as_esdt_identifier());
        arg_buffer.push_argument_bytes(self.blockchain().get_sc_address().as_bytes());
        arg_buffer.push_argument_bytes(owner.as_bytes());
        arg_buffer.push_argument_bytes(&total_fee_precent.to_be_bytes()[..]);
        arg_buffer.push_argument_bytes(&special_fee_precent.to_be_bytes()[..]);
        let new_address =
            self.send()
                .deploy_contract(gas_left, &amount, &code, code_metadata, &arg_buffer);
        if new_address != Address::zero() {
            self.pair_map().insert(
                PairTokens {
                    first_token_id: first_token_id.clone(),
                    second_token_id: second_token_id.clone(),
                },
                new_address.clone(),
            );
        }
        new_address
    }

    fn start_pair_construct(&self) {
        self.pair_code_ready().set(&false);
        self.pair_code().set(&BoxedBytes::empty());
    }

    fn end_pair_construct(&self) {
        self.pair_code_ready().set(&true);
    }

    fn append_pair_code(&self, part: &BoxedBytes) {
        let existent = self.pair_code().get();
        let new_code = BoxedBytes::from_concat(&[existent.as_slice(), part.as_slice()]);
        self.pair_code().set(&new_code);
    }

    fn upgrade_pair(&self, _address: &Address) {
        //TODO
    }

    #[storage_mapper("pair_map")]
    fn pair_map(&self) -> MapMapper<Self::Storage, PairTokens, Address>;

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

    #[view(getPairCode)]
    #[storage_mapper("pair_code")]
    fn pair_code(&self) -> SingleValueMapper<Self::Storage, BoxedBytes>;

    #[view(getPairCodeReady)]
    #[storage_mapper("pair_code_ready")]
    fn pair_code_ready(&self) -> SingleValueMapper<Self::Storage, bool>;
}
