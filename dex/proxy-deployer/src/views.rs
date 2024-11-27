multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait ViewModule: crate::storage::StorageModule {
    #[view(getAddressForToken)]
    fn get_address_for_token(&self, token_id: TokenIdentifier) -> OptionalValue<ManagedAddress> {
        let mapper = self.address_for_token(&token_id);
        if mapper.is_empty() {
            return OptionalValue::None;
        }

        let id = mapper.get();
        let addr = self.get_by_id(&self.address_id(), id);

        OptionalValue::Some(addr)
    }

    #[view(getContractOwner)]
    fn get_contract_owner(
        &self,
        contract_address: ManagedAddress,
    ) -> OptionalValue<ManagedAddress> {
        let contract_id = self.address_id().get_id(&contract_address);
        if contract_id == 0 {
            return OptionalValue::None;
        }

        let owner_id = self.contract_owner(contract_id).get();
        let opt_address = self.address_id().get_address(owner_id);
        match opt_address {
            Some(addr) => OptionalValue::Some(addr),
            None => OptionalValue::None,
        }
    }

    /// Indexes start at 1
    #[view(getAllUsedTokens)]
    fn get_all_used_tokens(
        &self,
        start_index: usize,
        max_entries: usize,
    ) -> MultiValueEncoded<TokenIdentifier> {
        let mapper = self.all_used_tokens();
        self.get_entries(&mapper, start_index, max_entries)
    }

    /// Indexes start at 1
    #[view(getAllDeployedContractsBySc)]
    fn get_all_deployed_contracts_by_sc(
        &self,
        start_index: usize,
        max_entries: usize,
    ) -> MultiValueEncoded<ManagedAddress> {
        let id_mapper = self.address_id();
        let mapper = self.all_deployed_contracts();
        let contract_ids = self.get_entries(&mapper, start_index, max_entries);

        let mut result = MultiValueEncoded::new();
        for contract_id in contract_ids {
            let address = self.get_by_id(&id_mapper, contract_id);
            result.push(address);
        }

        result
    }

    #[view(getAllDeployedContractsByUser)]
    fn get_all_deployed_contracts_by_user(
        &self,
        user: ManagedAddress,
        start_index: usize,
        max_entries: usize,
    ) -> MultiValueEncoded<ManagedAddress> {
        let id_mapper = self.address_id();
        let user_id = id_mapper.get_id(&user);
        if user_id == 0 {
            return MultiValueEncoded::new();
        }

        let mapper = self.contracts_by_address(user_id);
        let contract_ids = self.get_entries(&mapper, start_index, max_entries);

        let mut result = MultiValueEncoded::new();
        for contract_id in contract_ids {
            let address = self.get_by_id(&id_mapper, contract_id);
            result.push(address);
        }

        result
    }

    fn get_entries<T: TopEncode + TopDecode + NestedEncode + NestedDecode + 'static>(
        &self,
        mapper: &UnorderedSetMapper<T>,
        start_index: usize,
        max_entries: usize,
    ) -> MultiValueEncoded<T> {
        require!(start_index > 0, "Invalid start index");

        let mut items = MultiValueEncoded::new();
        let mut current_index = start_index;
        let mapper_len = mapper.len();
        for _ in 0..max_entries {
            if current_index > mapper_len {
                break;
            }

            let current_item = mapper.get_by_index(current_index);
            items.push(current_item);

            current_index += 1;
        }

        items
    }

    fn get_by_id(&self, id_mapper: &AddressToIdMapper, id: AddressId) -> ManagedAddress {
        let opt_address = id_mapper.get_address(id);
        require!(opt_address.is_some(), "Invalid setup");

        unsafe { opt_address.unwrap_unchecked() }
    }
}
