use crate::deploy::ForcedDeployArg;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait StorageModule {
    #[view(getTemplateAddress)]
    #[storage_mapper("templateAddress")]
    fn template_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("addressId")]
    fn address_id(&self) -> AddressToIdMapper;

    #[storage_mapper("addrForTok")]
    fn address_for_token(&self, token_id: &TokenIdentifier) -> SingleValueMapper<AddressId>;

    #[storage_mapper("allUsedTokens")]
    fn all_used_tokens(&self) -> UnorderedSetMapper<TokenIdentifier>;

    #[storage_mapper("allDeployedContracts")]
    fn all_deployed_contracts(&self) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("forcedDeployArgs")]
    fn forced_deploy_args(&self) -> SingleValueMapper<ManagedVec<ForcedDeployArg<Self::Api>>>;
}
