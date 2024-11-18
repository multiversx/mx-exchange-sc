use common_structs::Percent;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi)]
pub struct PairTokens<M: ManagedTypeApi> {
    pub first_token_id: TokenIdentifier<M>,
    pub second_token_id: TokenIdentifier<M>,
}

pub struct CreatePairArgs<'a, M: ManagedTypeApi> {
    pub first_token_id: &'a TokenIdentifier<M>,
    pub second_token_id: &'a TokenIdentifier<M>,
    pub owner: &'a ManagedAddress<M>,
    pub total_fee_percent: Percent,
    pub special_fee_percent: Percent,
    pub initial_liquidity_adder: &'a ManagedAddress<M>,
    pub admins: MultiValueEncoded<M, ManagedAddress<M>>,
}

#[multiversx_sc::module]
pub trait CreateModule:
    crate::config::ConfigModule
    + pair::read_pair_storage::ReadPairStorageModule
    + crate::temp_owner::TempOwnerModule
{
    fn create_pair(&self, args: CreatePairArgs<Self::Api>) -> ManagedAddress {
        require!(
            !self.pair_template_address().is_empty(),
            "pair contract template is empty"
        );

        let (new_address, ()) = self
            .pair_contract_deploy_proxy()
            .init(
                args.first_token_id,
                args.second_token_id,
                self.blockchain().get_sc_address(),
                args.owner,
                args.total_fee_percent,
                args.special_fee_percent,
                args.initial_liquidity_adder,
                args.admins,
            )
            .deploy_from_source(
                &self.pair_template_address().get(),
                CodeMetadata::UPGRADEABLE | CodeMetadata::READABLE | CodeMetadata::PAYABLE_BY_SC,
            );

        self.pair_map().insert(
            PairTokens {
                first_token_id: args.first_token_id.clone(),
                second_token_id: args.second_token_id.clone(),
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
        new_address
    }

    #[proxy]
    fn pair_contract_deploy_proxy(&self) -> pair::Proxy<Self::Api>;
}
