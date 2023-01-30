#![no_std]

multiversx_sc::imports!();

pub static CANNOT_UNWRAP_MSG: &[u8] = b"Cannot unwrap value";

pub trait Unwrappable<T> {
    fn unwrap_or_panic<M: ManagedTypeApi>(self) -> T;
}

impl<T> Unwrappable<T> for Option<T> {
    fn unwrap_or_panic<M: ManagedTypeApi>(self) -> T {
        self.unwrap_or_else(|| M::error_api_impl().signal_error(CANNOT_UNWRAP_MSG))
    }
}

impl<T, E> Unwrappable<T> for Result<T, E> {
    fn unwrap_or_panic<M: ManagedTypeApi>(self) -> T {
        self.unwrap_or_else(|_| M::error_api_impl().signal_error(CANNOT_UNWRAP_MSG))
    }
}
