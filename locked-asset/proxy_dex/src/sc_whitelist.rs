multiversx_sc::imports!();

#[multiversx_sc::module]
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

    #[only_owner]
    #[endpoint(addFarmToIntermediate)]
    fn add_farm_to_intermediate(&self, farm_address: ManagedAddress) {
        let _ = self.intermediated_farms().insert(farm_address);
    }

    #[only_owner]
    #[endpoint(removeIntermediatedFarm)]
    fn remove_intermediated_farm(&self, farm_address: ManagedAddress) {
        self.require_is_intermediated_farm(&farm_address);
        let _ = self.intermediated_farms().swap_remove(&farm_address);
    }

    fn require_is_intermediated_pair(&self, address: &ManagedAddress) {
        require!(
            self.intermediated_pairs().contains(address),
            "Not an intermediated pair"
        );
    }

    fn require_is_intermediated_farm(&self, address: &ManagedAddress) {
        require!(
            self.intermediated_farms().contains(address),
            "Not an intermediated farm"
        );
    }

    #[view(getIntermediatedPairs)]
    #[storage_mapper("intermediatedPairs")]
    fn intermediated_pairs(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[view(getIntermediatedFarms)]
    #[storage_mapper("intermediatedFarms")]
    fn intermediated_farms(&self) -> UnorderedSetMapper<ManagedAddress>;
}
