pub static BAD_LOCKING_TOKEN: &[u8] = b"wrong token for locking";
pub static CALLER_NOTHING_TO_CLAIM: &[u8] = b"caller has nothing to claim";
pub static CALLER_ON_COOLDOWN: &[u8] = b"caller cannot use this contract at this time";
pub static TOKENS_STILL_LOCKED: &[u8] = b"requested funds are still locked";
pub static ALREADY_SENT_TO_ADDRESS: &[u8] =
    b"caller already sent unclaimed funds to the destination address";
pub static TRANSFER_NON_EXISTENT: &[u8] = b"The transfer does not exist";
pub static ADDRESS_BLACKLISTED: &[u8] = b"The address is blacklisted";
pub static ADDRESS_NOT_BLACKLISTED: &[u8] = b"The address is not blacklisted";
