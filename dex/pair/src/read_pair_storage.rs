use multiversx_sc::storage::StorageKey;

use crate::safe_price::PriceObservation;

multiversx_sc::imports!();

pub static LP_TOKEN_SUPPLY_STORAGE_KEY: &[u8] = b"lp_token_supply";
pub static FIRST_TOKEN_ID_STORAGE_KEY: &[u8] = b"first_token_id";
pub static SECOND_TOKEN_ID_STORAGE_KEY: &[u8] = b"second_token_id";
pub static SAFE_PRICE_CURRENT_INDEX_STORAGE_KEY: &[u8] = b"safe_price_current_index";
pub static PRICE_OBSERVATIONS_STORAGE_KEY: &[u8] = b"price_observations";
pub static PAIR_RESERVE_BASE_STORAGE_KEY: &[u8] = b"reserve";

#[multiversx_sc::module]
pub trait ReadPairStorageModule {
    fn get_lp_token_supply_mapper(
        &self,
        pair_address: ManagedAddress,
    ) -> SingleValueMapper<BigUint, ManagedAddress> {
        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            pair_address,
            StorageKey::new(LP_TOKEN_SUPPLY_STORAGE_KEY),
        )
    }

    fn get_first_token_id_mapper(
        &self,
        pair_address: ManagedAddress,
    ) -> SingleValueMapper<TokenIdentifier, ManagedAddress> {
        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            pair_address,
            StorageKey::new(FIRST_TOKEN_ID_STORAGE_KEY),
        )
    }

    fn get_second_token_id_mapper(
        &self,
        pair_address: ManagedAddress,
    ) -> SingleValueMapper<TokenIdentifier, ManagedAddress> {
        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            pair_address,
            StorageKey::new(SECOND_TOKEN_ID_STORAGE_KEY),
        )
    }

    fn get_safe_price_current_index_mapper(
        &self,
        pair_address: ManagedAddress,
    ) -> SingleValueMapper<usize, ManagedAddress> {
        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            pair_address,
            StorageKey::new(SAFE_PRICE_CURRENT_INDEX_STORAGE_KEY),
        )
    }

    fn get_price_observation_mapper(
        &self,
        pair_address: ManagedAddress,
    ) -> VecMapper<PriceObservation<Self::Api>, ManagedAddress> {
        VecMapper::<_, _, ManagedAddress>::new_from_address(
            pair_address,
            StorageKey::new(PRICE_OBSERVATIONS_STORAGE_KEY),
        )
    }

    fn get_pair_reserve_mapper(
        &self,
        pair_address: ManagedAddress,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<BigUint, ManagedAddress> {
        let mut storage_key = StorageKey::new(PAIR_RESERVE_BASE_STORAGE_KEY);
        storage_key.append_item(&token_id);

        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(pair_address, storage_key)
    }
}
