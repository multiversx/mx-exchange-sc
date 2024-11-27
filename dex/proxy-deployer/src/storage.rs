multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, Clone, Copy, PartialEq)]
pub enum DeployerType {
    None,
    FarmStaking,
    FarmWithTopUp,
}

#[multiversx_sc::module]
pub trait StorageModule {
    fn get_by_id(&self, id_mapper: &AddressToIdMapper, id: AddressId) -> ManagedAddress {
        let opt_address = id_mapper.get_address(id);
        require!(opt_address.is_some(), "Invalid setup");

        unsafe { opt_address.unwrap_unchecked() }
    }

    #[view(getDeployerType)]
    #[storage_mapper("deployerType")]
    fn deployer_type(&self) -> SingleValueMapper<DeployerType>;

    #[view(getTemplateAddress)]
    #[storage_mapper("templateAddress")]
    fn template_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("addressId")]
    fn address_id(&self) -> AddressToIdMapper;

    #[storage_mapper("contractOwner")]
    fn contract_owner(&self, contract_id: AddressId) -> SingleValueMapper<AddressId>;

    #[storage_mapper("contractByAddress")]
    fn contracts_by_address(&self, address_id: AddressId) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("addrForTok")]
    fn address_for_token(&self, token_id: &TokenIdentifier) -> SingleValueMapper<AddressId>;

    #[storage_mapper("tokForAddr")]
    fn token_for_address(&self, address_id: AddressId) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("allUsedTokens")]
    fn all_used_tokens(&self) -> UnorderedSetMapper<TokenIdentifier>;

    #[storage_mapper("allDeployedContracts")]
    fn all_deployed_contracts(&self) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("userBlacklist")]
    fn user_blacklist(&self) -> WhitelistMapper<AddressId>;

    #[storage_mapper("timestampOracleAddress")]
    fn timestamp_oracle_address(&self) -> SingleValueMapper<ManagedAddress>;
}
