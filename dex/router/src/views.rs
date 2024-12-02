use crate::pair_actions::create::PairTokens;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(ManagedVecItem, TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct PairContractMetadata<M: ManagedTypeApi> {
    pub first_token_id: TokenIdentifier<M>,
    pub second_token_id: TokenIdentifier<M>,
    pub address: ManagedAddress<M>,
}

#[multiversx_sc::module]
pub trait ViewsModule:
    crate::config::ConfigModule + pair::read_pair_storage::ReadPairStorageModule
{
    #[view(getAllPairsManagedAddresses)]
    fn get_all_pairs_addresses(&self) -> MultiValueEncoded<ManagedAddress> {
        let mut result = MultiValueEncoded::new();
        for pair in self.pair_map().values() {
            result.push(pair);
        }

        result
    }

    #[view(getAllPairTokens)]
    fn get_all_token_pairs(&self) -> MultiValueEncoded<PairTokens<Self::Api>> {
        let mut result = MultiValueEncoded::new();
        for pair in self.pair_map().keys() {
            result.push(pair);
        }

        result
    }

    #[view(getAllPairContractMetadata)]
    fn get_all_pair_contract_metadata(&self) -> MultiValueEncoded<PairContractMetadata<Self::Api>> {
        let mut result = MultiValueEncoded::new();
        for (k, v) in self.pair_map().iter() {
            let pair_metadata = PairContractMetadata {
                first_token_id: k.first_token_id,
                second_token_id: k.second_token_id,
                address: v,
            };
            result.push(pair_metadata);
        }

        result
    }

    #[view(getPair)]
    fn get_pair(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) -> ManagedAddress {
        let mut address = self
            .pair_map()
            .get(&PairTokens {
                first_token_id: first_token_id.clone(),
                second_token_id: second_token_id.clone(),
            })
            .unwrap_or_else(ManagedAddress::zero);

        if address.is_zero() {
            address = self
                .pair_map()
                .get(&PairTokens {
                    first_token_id: second_token_id,
                    second_token_id: first_token_id,
                })
                .unwrap_or_else(ManagedAddress::zero);
        }

        address
    }
}
