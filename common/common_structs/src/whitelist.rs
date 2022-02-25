elrond_wasm::imports!();

use core::marker::PhantomData;
use elrond_wasm::{api::StorageMapperApi, storage::StorageKey};

type FlagMapper<SA> = SingleValueMapper<SA, bool>;

pub struct Whitelist<SA, T>
where
    SA: StorageMapperApi,
    T: NestedEncode + 'static,
{
    base_key: StorageKey<SA>,
    _phantom: PhantomData<T>,
}

impl<SA, T> StorageMapper<SA> for Whitelist<SA, T>
where
    SA: StorageMapperApi,
    T: NestedEncode + 'static,
{
    fn new(base_key: StorageKey<SA>) -> Self {
        Self {
            base_key,
            _phantom: PhantomData,
        }
    }
}

impl<SA, T> Whitelist<SA, T>
where
    SA: StorageMapperApi,
    T: NestedEncode + 'static,
{
    pub fn contains(&self, item: &T) -> bool {
        let mapper = self.build_mapper_for_item(item);
        !mapper.is_empty()
    }

    pub fn add(&mut self, item: &T) {
        let mapper = self.build_mapper_for_item(item);
        mapper.set(&true);
    }

    pub fn remove(&mut self, item: &T) {
        let mapper = self.build_mapper_for_item(item);
        mapper.clear();
    }

    pub fn require_whitelisted(&self, item: &T) {
        if !self.contains(item) {
            SA::error_api_impl().signal_error(b"Item not whitelisted");
        }
    }

    fn build_mapper_for_item(&self, item: &T) -> FlagMapper<SA> {
        let mut key = self.base_key.clone();
        key.append_item(item);

        FlagMapper::<SA>::new(key)
    }
}
