#![no_std]

elrond_wasm::imports!();

use common_structs::{InitialOldLockedTokenAttributes, Nonce, OldLockedTokenAttributes};

pub const LOCKED_TOKEN_ACTIVATION_NONCE: u64 = 2_286_815u64;

#[elrond_wasm::module]
pub trait LegacyTokenDecodeModule: utils::UtilsModule {
    fn decode_legacy_token(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
    ) -> OldLockedTokenAttributes<Self::Api> {
        let attributes: OldLockedTokenAttributes<Self::Api> =
            if token_nonce < LOCKED_TOKEN_ACTIVATION_NONCE {
                let initial_attributes: InitialOldLockedTokenAttributes<Self::Api> =
                    self.get_token_attributes(token_id, token_nonce);
                initial_attributes.migrate_to_new_attributes()
            } else {
                let updated_attributes: OldLockedTokenAttributes<Self::Api> =
                    self.get_token_attributes(token_id, token_nonce);
                updated_attributes
            };

        attributes
    }
}
