// Pair constants

pub static PAIR_WASM_PATH: &str = "pair/output/pair.wasm";
pub static WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
pub static RIDE_TOKEN_ID: &[u8] = b"RIDE-abcdef";
pub static LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef"; // also farming token ID for LP farm

pub const USER_TOTAL_WEGLD_TOKENS: u64 = 2_000_000_000;
pub const USER_TOTAL_RIDE_TOKENS: u64 = 2_000_000_000;
pub const USER_TOTAL_LP_TOKENS: u64 = 1_001_000_000;

pub const BLOCK_NONCE_FIRST_ADD_LIQ: u64 = 5;
pub const BLOCK_NONCE_SECOND_ADD_LIQ: u64 = 6;
pub const BLOCK_NONCE_AFTER_PAIR_SETUP: u64 = 100;

pub const SAFE_PRICE_MAX_OBSERVATIONS: usize = 10;

// LP farm constants

pub static FARM_WASM_PATH: &str = "farm/output/farm.wasm";
pub static LP_FARM_TOKEN_ID: &[u8] = b"LPFARM-abcdef";
pub const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
pub const MIN_FARMING_EPOCHS: u64 = 2;
pub const PENALTY_PERCENT: u64 = 10;
pub const LP_FARM_PER_BLOCK_REWARD_AMOUNT: u64 = 5_000;

// Staking farm constants

pub static STAKING_FARM_WASM_PATH: &str = "farm-staking/output/farm-staking.wasm";
pub static STAKING_REWARD_TOKEN_ID: &[u8] = RIDE_TOKEN_ID;
pub static STAKING_TOKEN_ID: &[u8] = RIDE_TOKEN_ID;
pub static STAKING_FARM_TOKEN_ID: &[u8] = b"STKFARM-abcdef";
pub const MAX_APR: u64 = 5_000; // 50%
pub const UNBOND_EPOCHS: u64 = 10;
pub const STAKING_FARM_PER_BLOCK_REWARD_AMOUNT: u64 = 1_000;
pub const REWARD_CAPACITY: u64 = 1_000_000_000_000;
pub const USER_REWARDS_BASE_CONST: u64 = 10;
pub const USER_REWARDS_ENERGY_CONST: u64 = 3;
pub const USER_REWARDS_FARM_CONST: u64 = 2;
pub const MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;
pub const MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;

// Proxy constants

pub static PROXY_WASM_PATH: &str = "farm-staking-proxy/output/farm-staking-proxy";
pub static DUAL_YIELD_TOKEN_ID: &[u8] = b"DYIELD-abcdef";
