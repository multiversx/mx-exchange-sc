use crate::Blocks;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait TempOwnerModule {
    #[only_owner]
    #[endpoint(setTemporaryOwnerPeriod)]
    fn set_temporary_owner_period(&self, period_blocks: Blocks) {
        self.temporary_owner_period().set(period_blocks);
    }

    #[only_owner]
    #[endpoint(clearPairTemporaryOwnerStorage)]
    fn clear_pair_temporary_owner_storage(&self) -> usize {
        let size = self.pair_temporary_owner().len();
        self.pair_temporary_owner().clear();

        size
    }

    fn get_pair_temporary_owner(&self, pair_address: &ManagedAddress) -> Option<ManagedAddress> {
        let result = self.pair_temporary_owner().get(pair_address);
        match result {
            Some((temporary_owner, creation_block)) => {
                let expire_block = creation_block + self.temporary_owner_period().get();
                if expire_block <= self.blockchain().get_block_nonce() {
                    self.pair_temporary_owner().remove(pair_address);

                    None
                } else {
                    Some(temporary_owner)
                }
            }
            None => None,
        }
    }

    #[view(getTemporaryOwnerPeriod)]
    #[storage_mapper("temporary_owner_period")]
    fn temporary_owner_period(&self) -> SingleValueMapper<Blocks>;

    #[storage_mapper("pair_temporary_owner")]
    fn pair_temporary_owner(&self) -> MapMapper<ManagedAddress, (ManagedAddress, Blocks)>;
}
