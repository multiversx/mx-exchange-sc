pub const ERROR_NOT_ACTIVE: &[u8] = b"Not active";
pub const ERROR_LP_TOKEN_NOT_ISSUED: &[u8] = b"LP token not issued";

pub const ERROR_BAD_PAYMENT_TOKENS: &[u8] = b"Bad payment tokens";
pub const ERROR_ARGS_NOT_MATCH_PAYMENTS: &[u8] = b"Arguments do not match payments";

pub const ERROR_INVALID_PAYMENTS: &[u8] = b"Invalid payments";
pub const ERROR_INVALID_ARGS: &[u8] = b"Invalid args";

pub const ERROR_FIRST_LIQUDITY: &[u8] = b"First tokens needs to be greater than minimum liquidity";
pub const ERROR_INSUFFICIENT_LIQUIDITY: &[u8] = b"Insufficient liquidity minted";

pub const ERROR_INSUFFICIENT_FIRST_TOKEN: &[u8] = b"Insufficient first token computed amount";
pub const ERROR_INSUFFICIENT_SECOND_TOKEN: &[u8] = b"Insufficient second token computed amount";
pub const ERROR_OPTIMAL_GRATER_THAN_PAID: &[u8] = b"Optimal amount greater than desired amount";

pub const ERROR_K_INVARIANT_FAILED: &[u8] = b"K invariant failed";

pub const ERROR_INSUFFICIENT_LIQ_BURNED: &[u8] = b"Insufficient liquidity burned";
pub const ERROR_SLIPPAGE_ON_REMOVE: &[u8] = b"Slippage amount does not match";
pub const ERROR_NOT_ENOUGH_RESERVE: &[u8] = b"Not enough reserve";
pub const ERROR_NOT_ENOUGH_LP: &[u8] = b"Not enough LP token supply";
pub const ERROR_INITIAL_LIQUIDITY_NOT_ADDED: &[u8] = b"Initial liquidity was not added";
pub const ERROR_INITIAL_LIQUIDITY_ALREADY_ADDED: &[u8] = b"Initial liquidity was already added";

pub const ERROR_NOT_AN_ESDT: &[u8] = b"Not a valid esdt id";
pub const ERROR_SAME_TOKENS: &[u8] = b"Exchange tokens cannot be the same";
pub const ERROR_POOL_TOKEN_IS_PLT: &[u8] = b"Token ID cannot be the same as LP token ID";
pub const ERROR_BAD_PERCENTS: &[u8] = b"Bad percents";
pub const ERROR_PERMISSION_DENIED: &[u8] = b"Permission denied";
pub const ERROR_NOT_WHITELISTED: &[u8] = b"Not whitelisted";
pub const ERROR_ALREADY_WHITELISTED: &[u8] = b"Already whitelisted";
pub const ERROR_PAIR_ALREADY_TRUSTED: &[u8] = b"Pair already trusted";
pub const ERROR_PAIR_NOT_TRUSTED: &[u8] = b"Pair not trusted";

pub const ERROR_ALREADY_FEE_DEST: &[u8] = b"Already a fee destination";
pub const ERROR_NOT_FEE_DEST: &[u8] = b"Not a fee destination";
pub const ERROR_BAD_TOKEN_FEE_DEST: &[u8] = b"Destination fee token differs";

pub const ERROR_ZERO_AMOUNT: &[u8] = b"Zero amount";
pub const ERROR_UNKNOWN_TOKEN: &[u8] = b"Unknown token";
pub const ERROR_LP_TOKEN_SAME_AS_POOL_TOKENS: &[u8] =
    b"LP token should differ from the exchange tokens";

pub const ERROR_SWAP_NOT_ENABLED: &[u8] = b"Swap is not enabled";
pub const ERROR_SLIPPAGE_EXCEEDED: &[u8] = b"Slippage exceeded";
pub const ERROR_NOTHING_TO_DO_WITH_FEE_SLICE: &[u8] = b"Nothing to do with fee slice";
