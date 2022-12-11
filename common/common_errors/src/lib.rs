#![no_std]

pub static ERROR_NOT_ACTIVE: &[u8] = b"Not active";
pub static ERROR_EMPTY_PAYMENTS: &[u8] = b"Empty payments";
pub static ERROR_TOO_MANY_ADDITIONAL_PAYMENTS: &[u8] = b"Too many additional payments";
pub static ERROR_BAD_INPUT_TOKEN: &[u8] = b"Bad input token";
pub static ERROR_NO_FARM_TOKEN: &[u8] = b"No farm token";
pub static ERROR_ZERO_AMOUNT: &[u8] = b"Zero amount";
pub static ERROR_NOT_AN_ESDT: &[u8] = b"Not a valid esdt id";
pub static ERROR_DIFFERENT_TOKEN_IDS: &[u8] = b"Different token ids";
pub static ERROR_SAME_TOKEN_IDS: &[u8] = b"Same token ids";
pub static ERROR_BAD_PAYMENTS_LEN: &[u8] = b"Bad payments len";
pub static ERROR_BAD_PAYMENTS: &[u8] = b"Bad payments";
pub static ERROR_NOT_ENOUGH_SUPPLY: &[u8] = b"Not enough supply";
pub static ERROR_NOT_A_FARM_TOKEN: &[u8] = b"Not a farm token";
pub static ERROR_NO_TOKEN_TO_MERGE: &[u8] = b"No token to merge";
pub static ERROR_PAYMENT_FAILED: &[u8] = b"Payment failed";
pub static ERROR_PERMISSION_DENIED: &[u8] = b"Permission denied";
pub static ERROR_PARAMETERS: &[u8] = b"Bad parameters";
