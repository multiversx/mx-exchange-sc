multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{enable_swap_by_user::EnableSwapByUserConfig, factory::PairTokens};
use pair::read_pair_storage;

#[multiversx_sc::module]
pub trait ConfigModule: read_pair_storage::ReadPairStorageModule {
    fn is_active(&self) -> bool {
        self.state().get()
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

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<bool>;

    #[view(getOwner)]
    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<ManagedAddress>;

    #[only_owner]
    #[endpoint(setTemporaryOwnerPeriod)]
    fn set_temporary_owner_period(&self, period_blocks: u64) {
        self.temporary_owner_period().set(period_blocks);
    }

    #[only_owner]
    #[endpoint(setPairTemplateAddress)]
    fn set_pair_template_address(&self, address: ManagedAddress) {
        self.pair_template_address().set(&address);
    }

    #[storage_mapper("pair_map")]
    fn pair_map(&self) -> MapMapper<PairTokens<Self::Api>, ManagedAddress>;

    #[view(getPairTemplateAddress)]
    #[storage_mapper("pair_template_address")]
    fn pair_template_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getTemporaryOwnerPeriod)]
    #[storage_mapper("temporary_owner_period")]
    fn temporary_owner_period(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("pair_temporary_owner")]
    fn pair_temporary_owner(&self) -> MapMapper<ManagedAddress, (ManagedAddress, u64)>;

    #[storage_mapper("enableSwapByUserConfig")]
    fn enable_swap_by_user_config(
        &self,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<EnableSwapByUserConfig<Self::Api>>;

    #[view(getCommonTokensForUserPairs)]
    #[storage_mapper("commonTokensForUserPairs")]
    fn common_tokens_for_user_pairs(&self) -> UnorderedSetMapper<TokenIdentifier>;
}
