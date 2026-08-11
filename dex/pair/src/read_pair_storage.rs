use multiversx_sc::storage::StorageKey;

use crate::safe_price::PriceObservation;

multiversx_sc::imports!();

pub static LP_TOKEN_SUPPLY_STORAGE_KEY: &[u8] = b"lp_token_supply";
pub static LP_TOKEN_ID_STORAGE_KEY: &[u8] = b"lpTokenIdentifier";
pub static FIRST_TOKEN_ID_STORAGE_KEY: &[u8] = b"first_token_id";
pub static SECOND_TOKEN_ID_STORAGE_KEY: &[u8] = b"second_token_id";
pub static SAFE_PRICE_CURRENT_INDEX_STORAGE_KEY: &[u8] = b"safe_price_current_index";
pub static PRICE_OBSERVATIONS_STORAGE_KEY: &[u8] = b"price_observations";
pub static CURRENT_PRICE_OBSERVATION_STORAGE_KEY: &[u8] = b"current_price_observation";
pub static SAFE_PRICE_LEGACY_CUTOVER_STORAGE_KEY: &[u8] = b"safe_price_legacy_cutover";
pub static PAIR_RESERVE_BASE_STORAGE_KEY: &[u8] = b"reserve";
pub static PAIR_ROUTER_STORAGE_KEY: &[u8] = b"router_address";

// Router storage keys
pub static SAFE_PRICE_TIMESTAMP_SAVE_INTERVAL_STORAGE_KEY: &[u8] =
    b"safe_price_timestamp_save_interval";
pub static DEFAULT_SAFE_PRICE_TIMESTAMP_OFFSET_STORAGE_KEY: &[u8] =
    b"default_safe_price_timestamp_offset";

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

    fn get_pair_lp_token_id_mapper(
        &self,
        pair_address: ManagedAddress,
    ) -> SingleValueMapper<TokenIdentifier, ManagedAddress> {
        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            pair_address,
            StorageKey::new(LP_TOKEN_ID_STORAGE_KEY),
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

    fn get_current_price_observation_mapper(
        &self,
        pair_address: ManagedAddress,
    ) -> SingleValueMapper<PriceObservation<Self::Api>, ManagedAddress> {
        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            pair_address,
            StorageKey::new(CURRENT_PRICE_OBSERVATION_STORAGE_KEY),
        )
    }

    fn get_safe_price_legacy_cutover_mapper(
        &self,
        pair_address: ManagedAddress,
    ) -> SingleValueMapper<(u64, u64), ManagedAddress> {
        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            pair_address,
            StorageKey::new(SAFE_PRICE_LEGACY_CUTOVER_STORAGE_KEY),
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

    fn get_pair_router_mapper(
        &self,
        pair_address: ManagedAddress,
    ) -> SingleValueMapper<ManagedAddress, ManagedAddress> {
        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            pair_address,
            StorageKey::new(PAIR_ROUTER_STORAGE_KEY),
        )
    }

    fn get_safe_price_timestamp_save_interval_mapper(
        &self,
        router_address: ManagedAddress,
    ) -> SingleValueMapper<u64, ManagedAddress> {
        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            router_address,
            StorageKey::new(SAFE_PRICE_TIMESTAMP_SAVE_INTERVAL_STORAGE_KEY),
        )
    }

    fn get_default_safe_price_timestamp_offset_mapper(
        &self,
        router_address: ManagedAddress,
    ) -> SingleValueMapper<u64, ManagedAddress> {
        SingleValueMapper::<_, _, ManagedAddress>::new_from_address(
            router_address,
            StorageKey::new(DEFAULT_SAFE_PRICE_TIMESTAMP_OFFSET_STORAGE_KEY),
        )
    }
}
