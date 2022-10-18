#![no_std]

elrond_wasm::imports!();

use common_structs::PaymentsVec;
use elrond_wasm::api::RawHandle;

static mut ALL_PAYMENTS_HANDLE: RawHandle = i32::MIN;

#[elrond_wasm::module]
pub trait StaticPaymentsModule {
    fn init_static_payments_from_api(&self) {
        let payments = self.call_value().all_esdt_transfers();
        unsafe {
            // must clone, as the framework uses the same handle
            let cloned = payments.clone();
            ALL_PAYMENTS_HANDLE = cloned.get_raw_handle();
        }
    }

    fn init_static_payments_from_value(&self, payments: PaymentsVec<Self::Api>) {
        unsafe {
            // just in case the value from API is sent as argument
            let cloned = payments.clone();
            ALL_PAYMENTS_HANDLE = cloned.get_raw_handle();
        }
    }

    fn get_static_payments(&self) -> PaymentsVec<Self::Api> {
        unsafe {
            require!(ALL_PAYMENTS_HANDLE != i32::MIN, "Payments not initialized");
        }

        let original_vec = unsafe { PaymentsVec::from_raw_handle(ALL_PAYMENTS_HANDLE) };

        // clone for safety, we don't want caller to modify what's behind the original handle
        original_vec.clone()
    }

    fn get_first_payment_from_static(&self) -> EsdtTokenPayment {
        let all_payments = self.get_static_payments();
        all_payments.get(0)
    }
}
