#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use elrond_wasm::HexCallDataSerializer;

// Temporary until the next version is released
// In 0.19, this entire file will be removed

pub const ESDT_MULTI_TRANSFER_STRING: &[u8] = b"MultiESDTNFTTransfer";

extern "C" {
    fn bigIntNew(value: i64) -> i32;
    fn bigIntUnsignedByteLength(x: i32) -> i32;
    fn bigIntGetUnsignedBytes(reference: i32, byte_ptr: *mut u8) -> i32;

    fn getNumESDTTransfers() -> i32;
    fn bigIntGetESDTCallValueByIndex(dest: i32, index: i32);
    fn getESDTTokenNameByIndex(resultOffset: *const u8, index: i32) -> i32;
    fn getESDTTokenNonceByIndex(index: i32) -> i64;
    fn getESDTTokenTypeByIndex(index: i32) -> i32;
}

pub struct EsdtTokenPayment<BigUint: BigUintApi> {
    pub token_type: EsdtTokenType,
    pub token_name: TokenIdentifier,
    pub token_nonce: u64,
    pub amount: BigUint,
}

#[elrond_wasm::module]
pub trait MultiTransferModule {
    fn esdt_num_transfers(&self) -> usize {
        unsafe { getNumESDTTransfers() as usize }
    }

    fn esdt_value_by_index(&self, index: usize) -> Self::BigUint {
        unsafe {
            let value_handle = bigIntNew(0);
            bigIntGetESDTCallValueByIndex(value_handle, index as i32);

            let mut value_buffer = [0u8; 64];
            let value_byte_len = bigIntUnsignedByteLength(value_handle) as usize;
            bigIntGetUnsignedBytes(value_handle, value_buffer.as_mut_ptr());

            Self::BigUint::from_bytes_be(&value_buffer[..value_byte_len])
        }
    }

    fn token_by_index(&self, index: usize) -> TokenIdentifier {
        unsafe {
            let mut name_buffer = [0u8; 32];
            let name_len = getESDTTokenNameByIndex(name_buffer.as_mut_ptr(), index as i32);
            if name_len == 0 {
                TokenIdentifier::egld()
            } else {
                TokenIdentifier::from(&name_buffer[..name_len as usize])
            }
        }
    }

    fn esdt_token_nonce_by_index(&self, index: usize) -> u64 {
        unsafe { getESDTTokenNonceByIndex(index as i32) as u64 }
    }

    fn esdt_token_type_by_index(&self, index: usize) -> EsdtTokenType {
        unsafe { (getESDTTokenTypeByIndex(index as i32) as u8).into() }
    }

    fn get_all_esdt_transfers(&self) -> Vec<EsdtTokenPayment<Self::BigUint>> {
        let num_transfers = self.esdt_num_transfers();
        let mut transfers = Vec::with_capacity(num_transfers);

        for i in 0..num_transfers {
            let token_type = self.esdt_token_type_by_index(i);
            let token_name = self.token_by_index(i);
            let token_nonce = self.esdt_token_nonce_by_index(i);
            let amount = self.esdt_value_by_index(i);

            transfers.push(EsdtTokenPayment {
                token_type,
                token_name,
                token_nonce,
                amount,
            });
        }

        transfers
    }

    fn multi_transfer_via_async_call(
        &self,
        to: &Address,
        transfers: &[EsdtTokenPayment<Self::BigUint>],
        endpoint_name: &BoxedBytes,
        args: &[BoxedBytes],
        callback_name: &BoxedBytes,
        callback_args: &[BoxedBytes],
    ) -> ! {
        let mut serializer = HexCallDataSerializer::new(ESDT_MULTI_TRANSFER_STRING);
        serializer.push_argument_bytes(to.as_bytes());
        serializer.push_argument_bytes(&transfers.len().to_be_bytes()[..]);

        for transf in transfers {
            serializer.push_argument_bytes(transf.token_name.as_esdt_identifier());
            serializer.push_argument_bytes(&transf.token_nonce.to_be_bytes()[..]);
            serializer.push_argument_bytes(transf.amount.to_bytes_be().as_slice());
        }

        if !endpoint_name.is_empty() {
            serializer.push_argument_bytes(endpoint_name.as_slice());

            for arg in args {
                serializer.push_argument_bytes(arg.as_slice());
            }
        }

        if !callback_name.is_empty() {
            let mut callback_data_serializer = HexCallDataSerializer::new(callback_name.as_slice());

            for cb_arg in callback_args {
                callback_data_serializer.push_argument_bytes(cb_arg.as_slice());
            }

            self.send()
                .storage_store_tx_hash_key(callback_data_serializer.as_slice());
        }

        self.send().async_call_raw(
            &self.blockchain().get_sc_address(),
            &Self::BigUint::zero(),
            serializer.as_slice(),
        );
    }
}
