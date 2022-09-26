elrond_wasm::imports!();

use common_structs::Nonce;

#[elrond_wasm::module]
pub trait OldTokenNonces {
    #[inline]
    fn is_new_token(&self, token_nonce: Nonce) -> bool {
        !self.old_token_nonces().contains(&token_nonce)
    }

    fn require_new_token(&self, token_nonce: Nonce) {
        require!(self.is_new_token(token_nonce), "Only new tokens accepted");
    }

    #[view(getOldTokenNonces)]
    #[storage_mapper("oldTokenNonces")]
    fn old_token_nonces(&self) -> UnorderedSetMapper<Nonce>;
}
