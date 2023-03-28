#![no_std]

multiversx_sc::imports!();

use common_structs::{InitialOldLockedTokenAttributes, Nonce, OldLockedTokenAttributes};

pub const LOCKED_TOKEN_ACTIVATION_NONCE: u64 = 2_286_815u64;

#[multiversx_sc::module]
pub trait LegacyTokenDecodeModule {
    fn decode_legacy_token(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
    ) -> OldLockedTokenAttributes<Self::Api> {
        if token_nonce < LOCKED_TOKEN_ACTIVATION_NONCE {
            let initial_attributes: InitialOldLockedTokenAttributes<Self::Api> = self
                .blockchain()
                .get_token_attributes(token_id, token_nonce);
            initial_attributes.migrate_to_new_attributes()
        } else {
            self.blockchain()
                .get_token_attributes(token_id, token_nonce)
        }
    }
}
