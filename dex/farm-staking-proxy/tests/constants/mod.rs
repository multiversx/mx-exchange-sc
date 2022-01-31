
// pair constants
pub const PAIR_WASM_PATH: &'static str = "pair/output/pair.wasm";
pub const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
pub const RIDE_TOKEN_ID: &[u8] = b"RIDE-abcdef";
pub const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef"; // also farming token ID for LP farm

pub const USER_TOTAL_WEGLD_TOKENS: u64 = 5_000_000_000;
pub const USER_TOTAL_RIDE_TOKENS: u64 = 5_000_000_000;

// LP farm constants

pub const FARM_WASM_PATH: &'static str = "farm/output/farm.wasm";
pub const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef"; // reward token ID
pub const LP_FARM_TOKEN_ID: &[u8] = b"LPFARM-abcdef";
pub const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
pub const MIN_FARMING_EPOCHS: u8 = 2;
pub const PENALTY_PERCENT: u64 = 10;
pub const PER_BLOCK_REWARD_AMOUNT: u64 = 5_000;
pub const USER_TOTAL_LP_TOKENS: u64 = 5_000_000_000;

// Staking farm constants

pub const STAKING_FARM_WASM_PATH: &str = "farm-staking/output/farm-staking.wasm";
pub const STAKING_REWARD_TOKEN_ID: &[u8] = RIDE_TOKEN_ID;
pub const STAKING_TOKEN_ID: &[u8] = RIDE_TOKEN_ID;
pub const STAKING_FARM_TOKEN_ID: &[u8] = b"STKFARM-abcdef";
pub const MAX_APR: u64 = 5_000; // 50%
pub const UNBOND_EPOCHS: u64 = 10;

// Proxy constants

pub const PROXY_WASM_PATH: &str = "farm-staking-proxy/output/farm-staking-proxy";
