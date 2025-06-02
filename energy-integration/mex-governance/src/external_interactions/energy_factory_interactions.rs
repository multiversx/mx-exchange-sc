multiversx_sc::imports!();

use multiversx_sc::storage::StorageKey;

use crate::errors::FARM_NOT_WHITELISTED;

pub static UNLOCKED_TOKEN_MINT_WHITELIST_STORAGE_KEY: &[u8] = b"unlockedTokenMintWhitelist";

#[multiversx_sc::module]
pub trait EnergyFactoryInteractionsModule: energy_query::EnergyQueryModule {
    fn get_unlocked_token_mint_whitelist_mapper(
        &self,
        sc_address: ManagedAddress,
    ) -> UnorderedSetMapper<ManagedAddress, ManagedAddress> {
        UnorderedSetMapper::<_, _, ManagedAddress>::new_from_address(
            sc_address,
            StorageKey::new(UNLOCKED_TOKEN_MINT_WHITELIST_STORAGE_KEY),
        )
    }

    fn check_farm_is_whitelisted(&self, farm_address: &ManagedAddress) {
        let energy_factory_address = self.energy_factory_address().get();
        let unlocked_token_mint_whitelist_mapper =
            self.get_unlocked_token_mint_whitelist_mapper(energy_factory_address);
        require!(
            unlocked_token_mint_whitelist_mapper.contains(farm_address),
            FARM_NOT_WHITELISTED
        );
    }
}
