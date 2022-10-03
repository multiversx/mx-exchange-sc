#![no_std]

elrond_wasm::imports!();

pub static CANNOT_MERGE_ERR_MSG: &[u8] = b"Cannot merge";

pub trait Mergeable<M: ManagedTypeApi> {
    fn error_if_not_mergeable(&self, other: &Self) {
        if !self.can_merge_with(&other) {
            M::error_api_impl().signal_error(CANNOT_MERGE_ERR_MSG);
        }
    }

    fn can_merge_with(&self, other: &Self) -> bool;

    fn merge_with(&mut self, other: Self);
}

impl<M: ManagedTypeApi> Mergeable<M> for EsdtTokenPayment<M> {
    fn can_merge_with(&self, other: &Self) -> bool {
        let same_token_id = self.token_identifier == other.token_identifier;
        let same_token_nonce = self.token_nonce == other.token_nonce;

        same_token_id && same_token_nonce
    }

    fn merge_with(&mut self, other: Self) {
        self.error_if_not_mergeable(&other);

        self.amount += other.amount;
    }
}
