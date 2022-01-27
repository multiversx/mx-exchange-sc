#![no_std]

pub const ERROR_NOT_ACTIVE: &[u8] = b"Not active";
pub const ERROR_NOT_MIGRATION: &[u8] = b"Not migration";
pub const ERROR_EMPTY_PAYMENTS: &[u8] = b"Empty payments";
pub const ERROR_BAD_INPUT_TOKEN: &[u8] = b"Bad input token";
pub const ERROR_NO_FARM_TOKEN: &[u8] = b"No farm token";
pub const ERROR_ZERO_AMOUNT: &[u8] = b"Zero amount";
pub const ERROR_NOT_AN_ESDT: &[u8] = b"Not a valid esdt id";
pub const ERROR_DIFFERENT_TOKEN_IDS: &[u8] = b"Different token ids";
pub const ERROR_SAME_TOKEN_IDS: &[u8] = b"Same token ids";
pub const ERROR_BAD_PAYMENTS_LEN: &[u8] = b"Bad payments len";
pub const ERROR_BAD_PAYMENTS: &[u8] = b"Bad payments";
pub const ERROR_NOT_ENOUGH_SUPPLY: &[u8] = b"Not enough supply";
pub const ERROR_NOT_A_FARM_TOKEN: &[u8] = b"Not a farm token";
pub const ERROR_NO_TOKEN_TO_MERGE: &[u8] = b"No token to merge";
pub const ERROR_PAYMENT_FAILED: &[u8] = b"Payment failed";
pub const ERROR_PERMISSIONS: &[u8] = b"Permission denied";
pub const ERROR_PARAMETERS: &[u8] = b"Bad parameters";
