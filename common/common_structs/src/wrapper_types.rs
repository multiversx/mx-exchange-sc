multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use core::ops::Deref;

use unwrappable::Unwrappable;

static NOT_ENOUGH_RESULTS_ERR_MSG: &[u8] = b"Not enough results";
const FIRST_VEC_INDEX: usize = 0;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi, Eq)]
pub struct TokenPair<M: ManagedTypeApi> {
    pub first_token: TokenIdentifier<M>,
    pub second_token: TokenIdentifier<M>,
}

impl<M: ManagedTypeApi> TokenPair<M> {
    pub fn equals(&self, other: &TokenPair<M>) -> bool {
        self.first_token == other.first_token && self.second_token == other.second_token
    }
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi)]
pub struct NonceAmountPair<M: ManagedTypeApi> {
    pub nonce: u64,
    pub amount: BigUint<M>,
}

impl<M: ManagedTypeApi> NonceAmountPair<M> {
    #[inline]
    pub fn new(nonce: u64, amount: BigUint<M>) -> Self {
        NonceAmountPair { nonce, amount }
    }
}

#[derive(
    TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, Clone, Debug,
)]
pub struct EpochAmountPair<M: ManagedTypeApi> {
    pub epoch: u64,
    pub amount: BigUint<M>,
}

#[derive(Clone)]
pub struct PaymentAttributesPair<
    M: ManagedTypeApi,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
> {
    pub payment: EsdtTokenPayment<M>,
    pub attributes: T,
}

pub type RawResultsType<M> = MultiValueEncoded<M, ManagedBuffer<M>>;

pub struct RawResultWrapper<M: ManagedTypeApi> {
    raw_results: ManagedVec<M, ManagedBuffer<M>>,
}

impl<M: ManagedTypeApi> RawResultWrapper<M> {
    pub fn new(raw_results: RawResultsType<M>) -> Self {
        Self {
            raw_results: raw_results.into_vec_of_buffers(),
        }
    }

    pub fn trim_results_front(&mut self, size_after_trim: usize) {
        let current_len = self.raw_results.len();
        if current_len < size_after_trim {
            M::error_api_impl().signal_error(NOT_ENOUGH_RESULTS_ERR_MSG);
        }
        if current_len == size_after_trim {
            return;
        }

        let new_start_index = current_len - size_after_trim;
        let opt_new_raw_results = self.raw_results.slice(new_start_index, current_len);
        self.raw_results = opt_new_raw_results.unwrap_or_panic::<M>();
    }

    pub fn decode_next_result<T: TopDecode>(&mut self) -> T {
        if self.raw_results.is_empty() {
            M::error_api_impl().signal_error(NOT_ENOUGH_RESULTS_ERR_MSG);
        }

        let result = {
            let raw_buffer_ref = self.raw_results.get(FIRST_VEC_INDEX);
            let decode_result = T::top_decode(raw_buffer_ref.deref().clone());
            decode_result.unwrap_or_panic::<M>()
        };
        self.raw_results.remove(FIRST_VEC_INDEX);

        result
    }
}
