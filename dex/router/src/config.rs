multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{enable_swap_by_user::EnableSwapByUserConfig, factory::PairTokens};

#[multiversx_sc::module]
pub trait ConfigModule {
    fn is_active(&self) -> bool {
        self.state().get()
    }

    fn check_is_pair_sc(&self, pair_address: &ManagedAddress) {
        require!(
            self.address_pair_map().contains_key(pair_address),
            "Not a pair SC"
        );
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

    #[storage_mapper("address_pair_map")]
    fn address_pair_map(&self) -> MapMapper<ManagedAddress, PairTokens<Self::Api>>;

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
