elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use elrond_codec::TopEncode;

const INITIAL_SFT_AMOUNT: u32 = 1;

#[elrond_wasm::module]
pub trait TokenAttributesModule {
    fn get_or_create_nonce_for_attributes<T: TopEncode + NestedEncode>(
        &self,
        nft_mapper: &NonFungibleTokenMapper<Self::Api>,
        attributes: &T,
    ) -> u64 {
        let token_id = nft_mapper.get_token_id();
        let mut encoded_attributes = ManagedBuffer::new();
        attributes
            .dep_encode(&mut encoded_attributes)
            .unwrap_or_else(|err| sc_panic!(err.message_str()));

        let attributes_to_nonce_mapper =
            self.attributes_to_nonce_mapping(&token_id, &encoded_attributes);
        let existing_nonce = attributes_to_nonce_mapper.get();
        if existing_nonce != 0 {
            return existing_nonce;
        }

        let new_nonce = nft_mapper
            .nft_create(INITIAL_SFT_AMOUNT.into(), attributes)
            .token_nonce;
        attributes_to_nonce_mapper.set(&new_nonce);

        new_nonce
    }

    fn get_attributes_for_nonce<T: TopDecode>(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: u64,
    ) -> T {
        let raw_attributes = self
            .nonce_to_attributes_mapping(token_id, token_nonce)
            .get();

        T::top_decode(raw_attributes).unwrap_or_else(|err| sc_panic!(err.message_str()))
    }

    // TODO: Swap to TokenAttributes mapper from Rust framework on upgrade
    // Current version is bugged, so we use a custom implementation for now
    #[storage_mapper("nonceToAttributesMapping")]
    fn nonce_to_attributes_mapping(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: u64,
    ) -> SingleValueMapper<ManagedBuffer>;

    #[storage_mapper("attributesToNonceMapping")]
    fn attributes_to_nonce_mapping(
        &self,
        token_id: &TokenIdentifier,
        attributes: &ManagedBuffer,
    ) -> SingleValueMapper<u64>;
}
