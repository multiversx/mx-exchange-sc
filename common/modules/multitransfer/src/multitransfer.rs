#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

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

    fn multiTransferESDTNFTExecute(
        dstOffset: *const u8,
        numTokenTransfers: i32,
        tokenTransfersArgsLengthOffset: *const u8,
        tokenTransferDataOffset: *const u8,
        gasLimit: i64,
        functionOffset: *const u8,
        functionLength: i32,
        numArguments: i32,
        argumentsLengthOffset: *const u8,
        dataOffset: *const u8,
    ) -> i32;
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

    fn direct_multi_esdt_transfer_execute(
        &self,
        to: &Address,
        payments: &[EsdtTokenPayment<Self::BigUint>],
        gas_limit: u64,
        endpoint_name: &BoxedBytes,
        arg_buffer: &ArgBuffer,
    ) -> SCResult<()> {
        unsafe {
            let nr_transfers = payments.len();
            let mut transfer_arg_lengths = Vec::with_capacity(nr_transfers * 3);
            let mut transfer_args = Vec::new();

            for token in payments {
                let token_id_bytes = token.token_name.as_esdt_identifier();
                let nonce_bytes = &token.token_nonce.to_be_bytes()[..];
                let amount_bytes = token.amount.to_bytes_be();

                transfer_arg_lengths.push(token_id_bytes.len() as i32);
                transfer_arg_lengths.push(nonce_bytes.len() as i32);
                transfer_arg_lengths.push(amount_bytes.len() as i32);

                transfer_args.extend_from_slice(token_id_bytes);
                transfer_args.extend_from_slice(nonce_bytes);
                transfer_args.extend_from_slice(amount_bytes.as_slice());
            }

            let result = multiTransferESDTNFTExecute(
                to.as_ptr(),
                nr_transfers as i32,
                transfer_arg_lengths.as_ptr() as *const u8,
                transfer_args.as_ptr(),
                gas_limit as i64,
                endpoint_name.as_ptr(),
                endpoint_name.len() as i32,
                arg_buffer.num_args() as i32,
                arg_buffer.arg_lengths_bytes_ptr(),
                arg_buffer.arg_data_ptr(),
            );

            require!(result == 0, "multiTransferESDTNFTExecute failed");
            Ok(())
        }
    }
}
