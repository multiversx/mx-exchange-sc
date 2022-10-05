elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait ScWhitelistModule {
    #[only_owner]
    #[endpoint(addPairToIntermediate)]
    fn add_pair_to_intermediate(&self, pair_address: ManagedAddress) {
        let _ = self.intermediated_pairs().insert(pair_address);
    }

    #[only_owner]
    #[endpoint(removeIntermediatedPair)]
    fn remove_intermediated_pair(&self, pair_address: ManagedAddress) {
        self.require_is_intermediated_pair(&pair_address);
        let _ = self.intermediated_pairs().swap_remove(&pair_address);
    }

    fn require_is_intermediated_pair(&self, address: &ManagedAddress) {
        require!(
            self.intermediated_pairs().contains(address),
            "Not an intermediated pair"
        );
    }

    #[view(getIntermediatedPairs)]
    #[storage_mapper("intermediatedPairs")]
    fn intermediated_pairs(&self) -> UnorderedSetMapper<ManagedAddress>;
}
