multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::pair_actions::create::PairTokens;
use pair::read_pair_storage;

#[multiversx_sc::module]
pub trait ConfigModule: read_pair_storage::ReadPairStorageModule {
    #[only_owner]
    #[endpoint(setPairTemplateAddress)]
    fn set_pair_template_address(&self, address: ManagedAddress) {
        self.pair_template_address().set(&address);
    }

    fn check_is_pair_sc(&self, pair_address: &ManagedAddress) {
        let first_token_id = self.get_first_token_id_mapper(pair_address.clone()).get();
        let second_token_id = self.get_second_token_id_mapper(pair_address.clone()).get();

        let pair_tokens = PairTokens {
            first_token_id: first_token_id.clone(),
            second_token_id: second_token_id.clone(),
        };

        let mut pair_map_address_opt = self.pair_map().get(&pair_tokens);
        if pair_map_address_opt.is_none() {
            let reverse_pair_tokens = PairTokens {
                first_token_id: second_token_id.clone(),
                second_token_id: first_token_id.clone(),
            };
            pair_map_address_opt = self.pair_map().get(&reverse_pair_tokens);
        }

        require!(pair_map_address_opt.is_some(), "Not a pair SC");

        unsafe {
            let pair_map_address = pair_map_address_opt.unwrap_unchecked();
            require!(&pair_map_address == pair_address, "Not a pair SC");
        }
    }

    #[view(getPairCreationEnabled)]
    #[storage_mapper("pair_creation_enabled")]
    fn pair_creation_enabled(&self) -> SingleValueMapper<bool>;

    #[view(getOwner)]
    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("pair_map")]
    fn pair_map(&self) -> MapMapper<PairTokens<Self::Api>, ManagedAddress>;

    #[view(getPairTemplateAddress)]
    #[storage_mapper("pair_template_address")]
    fn pair_template_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getCommonTokensForUserPairs)]
    #[storage_mapper("commonTokensForUserPairs")]
    fn common_tokens_for_user_pairs(&self) -> UnorderedSetMapper<TokenIdentifier>;
}
