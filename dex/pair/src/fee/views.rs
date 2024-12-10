use common_structs::TokenPair;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ViewsModule: super::storage::StorageModule {
    #[view(getFeeState)]
    fn is_fee_enabled(&self) -> bool {
        !self.destination_map().is_empty() || !self.fees_collector_address().is_empty()
    }

    #[view(getFeeDestinations)]
    fn get_fee_destinations(&self) -> MultiValueEncoded<(ManagedAddress, TokenIdentifier)> {
        let mut result = MultiValueEncoded::new();
        for pair in self.destination_map().iter() {
            result.push((pair.0, pair.1))
        }

        result
    }

    #[view(getTrustedSwapPairs)]
    fn get_trusted_swap_pairs(&self) -> MultiValueEncoded<(TokenPair<Self::Api>, ManagedAddress)> {
        let mut result = MultiValueEncoded::new();
        for pair in self.trusted_swap_pair().iter() {
            result.push((pair.0, pair.1))
        }

        result
    }

    #[view(getWhitelistedManagedAddresses)]
    fn get_whitelisted_managed_addresses(&self) -> MultiValueEncoded<ManagedAddress> {
        let mut result = MultiValueEncoded::new();
        for pair in self.whitelist().iter() {
            result.push(pair);
        }

        result
    }
}
