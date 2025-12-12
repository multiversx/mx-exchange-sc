# Safe Price Mechanism Documentation

## Table of Contents

1. [Overview](#overview)
2. [Architecture and Deployment](#architecture-and-deployment)
3. [Protocol Upgrade: Time-Based Safe Price](#protocol-upgrade-time-based-safe-price)
4. [How It Works](#how-it-works)
5. [Price Observation Structure](#price-observation-structure)
6. [Recording Mechanisms](#recording-mechanisms)
7. [Available Endpoints](#available-endpoints)
8. [Configuration](#configuration)
9. [Usage Examples](#usage-examples)
10. [Technical Details](#technical-details)

## Overview

The Safe Price mechanism is a Time-Weighted Average Price (TWAP) oracle implementation designed to provide manipulation-resistant price data for DEX pairs. It records historical price observations at regular intervals and calculates time-weighted averages to prevent flash-loan attacks and other price manipulation attempts.

### Key Benefits

- **Manipulation Resistance**: TWAP calculations prevent single-transaction price manipulation
- **Historical Data**: Maintains up to 65,536 price observations
- **Flexible Queries**: Supports both round-based and timestamp-based price lookups
- **Gas Efficiency**: Configurable recording intervals optimize gas costs
- **Time-Independent**: Works across different blockchain block duration configurations

## Architecture and Deployment

### Central Safe Price View Contract

**IMPORTANT**: dApps should query safe price data through the **Central Safe Price View Contract**, not individual pair contracts. This is the only recommended approach for production use.

#### Why Use the Central View Contract

- **Unified Interface**: Single contract address for all safe price queries across all pairs
- **Pair Address as Parameter**: Query any pair by passing its address as a parameter
- **View-Only Operations**: All endpoints are read-only view functions with no gas costs
- **Stable Integration**: Contract address remains constant even as new pairs are added
- **Cross-Pair Queries**: Easily query multiple pairs without managing multiple contract addresses

#### How It Works

The Safe Price View contract reads price observation data directly from individual pair contract storage. When you call an endpoint with a `pair_address` parameter, it:

1. Reads the pair's safe price observations from storage
2. Performs all calculations (interpolation, weighted averaging)
3. Returns manipulation-resistant price data

All historical data remains stored in the pair contracts, while the view contract provides a centralized query interface.

#### Deployment

The Safe Price View contract is built as a separate WASM module (`wasm-safe-price-view`) that exposes view endpoints from the pair contract's `SafePriceViewModule`.

**Deployed Contract**: `safe-price-view.wasm`

#### Legacy Endpoints

Individual pair contracts still expose safe price endpoints (`updateAndGetSafePrice`, `updateAndGetTokensForGivenPositionWithSafePrice`) for backwards compatibility only. These are **not recommended** for new integrations.

### Important Security Consideration

**CRITICAL**: The Safe Price module retrieves data independently of the liquidity pool's state. Even if a pair smart contract is paused, the safe price module will continue to return data.

**For external contract integrations**: If your contract requires awareness of the pair's operational status, you must **manually check the liquidity pool's pause state** before using safe price data. The safe price mechanism does not enforce or reflect pause states.

This design allows price queries to remain available for informational purposes while giving integrating contracts full control over how they handle paused pool scenarios.

## Protocol Upgrade: Time-Based Safe Price

### Background

With the MultiversX protocol upgrade reducing block duration from 6 seconds to 0.6 seconds, the safe price mechanism needed to become time-independent rather than relying solely on round numbers.

### Key Changes

#### 1. Timestamp Support in Price Observations

The `PriceObservation` structure now includes a `recording_timestamp` field alongside `recording_round`:

```rust
pub struct PriceObservation {
    pub first_token_reserve_accumulated: BigUint,
    pub second_token_reserve_accumulated: BigUint,
    pub weight_accumulated: u64,
    pub recording_round: Round,
    pub recording_timestamp: Timestamp,
    pub lp_supply_accumulated: BigUint,
}
```

#### 2. New Timestamp-Based Endpoints

- `getSafePriceByTimestampOffset`: Get safe price using a timestamp offset
- `getLpTokensSafePriceByTimestampOffset`: Get LP token value using timestamp offset

These endpoints allow querying prices based on elapsed time (in seconds) rather than rounds, making the system robust to block duration changes.

**Note:** The timestamp offset endpoint (`getSafePriceByTimestampOffset`) does not have a default value. It must be provided as a parameter.

#### 3. Timestamp-to-Round Conversion

The mechanism includes dedicated logic to find equivalent rounds for given timestamps:

- **Binary Search**: Efficiently locates observations by timestamp
- **Linear Interpolation**: Calculates intermediate rounds when exact matches aren't found
- **Weighted Averaging**: Combines neighboring observations for precise calculations

#### 4. Intermediate Save Functionality

To optimize gas costs with faster block times, the system now supports:

- **Configurable Save Intervals**: Set how often observations are finalized
- **Intermediate Accumulation**: Accumulates price data between saves
- **Automatic Finalization**: Saves averaged observations when intervals are reached

## How It Works

### Recording Process

1. **On Each Swap/Liquidity Event**: The contract updates safe price data
2. **Interval Check**: Determines if a new observation should be recorded
3. **Accumulation**: Either updates intermediate observation or saves a new finalized observation
4. **Circular Buffer**: Maintains most recent observations (up to MAX_OBSERVATIONS)

### Calculation Method

Safe price uses time-weighted averaging:

```
Weighted Reserve = (Σ reserve_i × time_i) / (Σ time_i)
```

Where:
- `reserve_i` is the token reserve at observation i
- `time_i` is the duration (weight) of that observation
- The sum covers all observations in the specified time range

### Price Query Process

1. **Determine Time Range**: Either from offset or explicit start/end rounds
2. **Fetch Observations**: Get observations at start and end of range
3. **Calculate Weighted Amounts**: Compute time-weighted reserves
4. **Return Price**: Calculate exchange rate from weighted reserves

## Price Observation Structure

### Circular Buffer Storage

Price observations are stored in a **circular buffer** (circular list) with a maximum capacity of **65,536 observations** (2^16). This data structure provides:

- **Efficient Storage**: Automatically overwrites oldest data when capacity is reached
- **Fast Lookups**: Optimized for binary search operations
- **Predictable Memory**: Fixed maximum storage footprint
- **Rolling Window**: Always maintains the most recent observation history

The circular buffer ensures that the system continuously captures price data while maintaining a bounded memory footprint, making it suitable for long-running production environments.

### Observation Fields

Each price observation records:

- **first_token_reserve_accumulated**: Cumulative weighted first token reserve
- **second_token_reserve_accumulated**: Cumulative weighted second token reserve
- **weight_accumulated**: Total time weight (rounds elapsed)
- **recording_round**: Blockchain round when recorded
- **recording_timestamp**: Unix timestamp when recorded
- **lp_supply_accumulated**: Cumulative weighted LP token supply

### Weight Calculation

The weight of a `PriceObservation` represents the time duration it covers. Specifically:

**Weight = Current Round - Last Saved Round**

This weight is used to properly time-weight the reserves when calculating safe prices. For example, if the last observation was saved at round 1000 and the current round is 1010, the weight for the new observation would be 10 rounds.

This weighting ensures that longer time periods have proportionally greater influence on the final TWAP calculation, preventing manipulation through rapid price changes.

## Recording Mechanisms

### Immediate Save Mode (Interval = 1)

Default behavior where every update creates a new observation:

```rust
safe_price_round_save_interval = 1 (default)
```

- Observation saved on every swap/liquidity change
- Most accurate but higher gas costs
- Suitable for lower-frequency trading

### Intermediate Save Mode (Interval > 1)

Accumulates data over multiple rounds before saving:

```rust
safe_price_round_save_interval = 10
```

- Accumulates weighted data in `current_price_observation`
- Finalizes observation when interval rounds have passed
- Lower gas costs but slightly delayed observations
- Optimal for high-frequency trading (0.6s blocks)

## Available Endpoints

### Token Swap Price Endpoints

#### `getSafePriceByDefaultOffset`

Get safe price using the default configured offset.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `input_payment: EsdtTokenPayment` - Input token and amount

**Returns:**
- `EsdtTokenPayment` - Output token and amount

**Usage:**
```rust
// Get price for swapping 1000 WEGLD
let output = getSafePriceByDefaultOffset(pair_addr, EsdtTokenPayment(WEGLD, 0, 1000));
```

#### `getSafePriceByRoundOffset`

Get safe price using a specific round offset.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `round_offset: Round` - Number of rounds to look back
- `input_payment: EsdtTokenPayment` - Input token and amount

**Returns:**
- `EsdtTokenPayment` - Output token and amount

**Example:**
```rust
// Get price from 600 rounds ago
let output = getSafePriceByRoundOffset(pair_addr, 600, input);
```

#### `getSafePriceByTimestampOffset`

Get safe price using a timestamp offset.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `timestamp_offset: Timestamp` - Number of seconds to look back
- `input_payment: EsdtTokenPayment` - Input token and amount

**Returns:**
- `EsdtTokenPayment` - Output token and amount

**Example:**
```rust
// Get price from 1 hour ago (3600 seconds)
let output = getSafePriceByTimestampOffset(pair_addr, 3600, input);
```

**Note:** This endpoint is time-independent and works across block duration changes.

#### `getSafePrice`

Get safe price for a custom round range.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `start_round: Round` - Starting round
- `end_round: Round` - Ending round
- `input_payment: EsdtTokenPayment` - Input token and amount

**Returns:**
- `EsdtTokenPayment` - Output token and amount

### LP Token Value Endpoints

#### `getLpTokensSafePriceByDefaultOffset`

Get LP token value using the default offset.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `liquidity: BigUint` - Amount of LP tokens

**Returns:**
- `MultiValue2<EsdtTokenPayment, EsdtTokenPayment>` - First and second token amounts

**Example:**
```rust
// Get value of 1000 LP tokens
let (first_token, second_token) = getLpTokensSafePriceByDefaultOffset(pair_addr, 1000);
```

#### `getLpTokensSafePriceByRoundOffset`

Get LP token value using a specific round offset.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `round_offset: Round` - Number of rounds to look back
- `liquidity: BigUint` - Amount of LP tokens

**Returns:**
- `MultiValue2<EsdtTokenPayment, EsdtTokenPayment>` - First and second token amounts

#### `getLpTokensSafePriceByTimestampOffset`

Get LP token value using a timestamp offset.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `timestamp_offset: Timestamp` - Number of seconds to look back
- `liquidity: BigUint` - Amount of LP tokens

**Returns:**
- `MultiValue2<EsdtTokenPayment, EsdtTokenPayment>` - First and second token amounts

**Example:**
```rust
// Get LP value from 30 minutes ago (1800 seconds)
let (token1, token2) = getLpTokensSafePriceByTimestampOffset(pair_addr, 1800, lp_amount);
```

#### `getLpTokensSafePrice`

Get LP token value for a custom round range.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `start_round: Round` - Starting round
- `end_round: Round` - Ending round
- `liquidity: BigUint` - Amount of LP tokens

**Returns:**
- `MultiValue2<EsdtTokenPayment, EsdtTokenPayment>` - First and second token amounts

### Observation Query Endpoints

#### `getPriceObservation`

Get a specific price observation for a given round.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `search_round: Round` - The round to query

**Returns:**
- `PriceObservation` - The observation data (may be interpolated)

### Legacy Endpoints

#### `updateAndGetTokensForGivenPositionWithSafePrice`

Legacy endpoint that calls `getLpTokensSafePriceByDefaultOffset` on the pair itself.

#### `updateAndGetSafePrice`

Legacy endpoint that calls `getSafePriceByDefaultOffset` on the pair itself.

### View Endpoints

#### `getSafePriceCurrentIndex`

Returns the current index in the circular observation buffer.

**Returns:**
- `usize` - Current index (1-based)

#### `getSafePriceRoundSaveInterval`

Returns the configured round save interval.

**Returns:**
- `Round` - Number of rounds between saves

**Default Value:** `1` (immediate save on every update)

#### `getCurrentPriceObservation`

Returns the current intermediate observation (if any).

**Returns:**
- `PriceObservation` - The intermediate observation

#### `getDefaultSafePriceRoundsOffset`

Returns the default round offset for safe price queries.

**Returns:**
- `u64` - Default offset in rounds

**Default Value:** `600` rounds

## Configuration

### Owner-Only Configuration Endpoints

#### `setSafePriceRoundSaveInterval`

Set how frequently observations are saved.

**Available on:**
- **Router contract**: The Router SC now acts like a Safe Price global hub for general config. This function sets the interval for all pairs at once.

**Parameters:**
- `new_interval: Round` - Must be > 0

**Example:**
- For 6s blocks: `interval = 1` (save every round)
- For 0.6s blocks: `interval = 10` (save every 6 seconds)

#### `setDefaultSafePriceRoundsOffset`

Set the default lookback period for safe price queries.

**Available on:**
- **Router contract**: The same value applies to all existing pairs.

**Parameters:**
- `new_offset: u64` - Must be > 0

**Default Value:**
- 600 rounds (10 blocks/minute × 60 minutes = 1 hour at 6s blocks)
- For 0.6s blocks: Consider 6000 rounds (100 blocks/minute × 60 minutes)

### Constants

- **MAX_OBSERVATIONS**: 65,536 (2^16 records for optimized binary search)
- **DEFAULT_ROUND_SAVE_INTERVAL**: 1
- **OFFSET_PRECISION_FACTOR**: 1,000,000 (for internal calculations)

## Usage Examples

### Example 1: Get Current Safe Price (Default Offset)

```rust
// Query safe price with default 1-hour lookback
let pair_address = managed_address!(...);
let input = EsdtTokenPayment::new(
    TokenIdentifier::from("WEGLD-123456"),
    0,
    BigUint::from(1000u64)
);

let output = self.get_safe_price_by_default_offset(
    pair_address,
    input
);
// Returns: EsdtTokenPayment for MEX with calculated amount
```

### Example 2: Get Price from Specific Time Ago (Timestamp)

```rust
// Get price from 30 minutes ago using timestamp
let thirty_minutes_seconds = 30 * 60; // 1800 seconds
let output = self.get_safe_price_by_timestamp_offset(
    pair_address,
    thirty_minutes_seconds,
    input
);
```

### Example 3: Get LP Token Value

```rust
// Calculate value of LP tokens from 1 hour ago
let one_hour_seconds = 3600;
let lp_amount = BigUint::from(1000000u64);

let (first_token, second_token) = self.get_lp_tokens_safe_price_by_timestamp_offset(
    pair_address,
    one_hour_seconds,
    lp_amount
);
// Returns: (WEGLD payment, MEX payment)
```

### Example 4: Custom Time Range

```rust
// Get price between two specific rounds
let start_round = 1000;
let end_round = 2000;

let output = self.get_safe_price(
    pair_address,
    start_round,
    end_round,
    input
);
```

## Technical Details

### Binary Search Algorithm

The mechanism uses binary search to efficiently find observations:

1. **Round-based search**: O(log n) complexity for finding observations by round
2. **Timestamp-based search**: O(log n) complexity for finding observations by timestamp
3. **Circular buffer handling**: Properly handles wraparound when buffer is full

### Linear Interpolation

When exact observation matches aren't found, the system interpolates:

```rust
// Weighted average calculation
weighted_value = (left_value × left_weight + right_value × right_weight) / total_weight

// Where:
// left_weight = distance to right observation
// right_weight = distance to left observation
```

This ensures smooth price curves and accurate intermediate values.

### Timestamp-to-Round Conversion

The `find_equivalent_round_for_timestamp` function:

1. Binary searches for closest observation by timestamp
2. If exact match: returns that observation's round
3. If no exact match: finds neighboring observations
4. Interpolates the round number based on timestamp position
5. Returns interpolated round for use in price calculations

### Migration Compatibility

The system maintains backward compatibility:

- Old observations without timestamps are handled using a custom decoding logic
- `NestedDecode` implementation fills missing fields with defaults
- Timestamp = 0 for legacy observations
- Existing functionality preserved while adding new features

### Gas Optimization

The intermediate save functionality reduces circular buffer writes:

- **Without intermediate saves** (interval=1): Each swap writes a new observation to the circular buffer (VecMapper)
- **With intermediate saves** (interval=10): Swaps update an intermediate observation (SingleValueMapper), and only every ~10 rounds writes to the circular buffer
- **Trade-off**: Both modes write to storage on each swap, but intermediate mode reduces the frequency of VecMapper operations (which involve index calculations and potentially more complex storage patterns)
- Optimal for high-frequency trading on 0.6s blocks

### Price Manipulation Resistance

TWAP provides strong manipulation resistance:

1. **Time-weighted**: Single-transaction attacks have minimal impact
2. **Historical lookback**: Uses data from before potential attack
3. **Configurable periods**: Longer periods = stronger resistance
4. **No instant updates**: Prevents flash-loan exploits

### Edge Cases Handled

- **Empty observations**: Returns appropriate defaults or errors
- **Same round queries**: Prevented with validation
- **Out of range**: Validates requested rounds are available
- **Zero reserves**: Handled gracefully, no division by zero
- **Buffer wraparound**: Circular buffer logic maintains correct ordering

## Best Practices for Developers

1. **Check pair pause state**: If your contract needs to respect pair pause states, manually check the liquidity pool's pause status before using safe price data. Safe price continues to return data even when pairs are paused.
2. **Use timestamp offsets**: More robust to protocol changes than round offsets
3. **Choose appropriate lookback**: Longer periods = more manipulation-resistant but less reactive
4. **Handle errors**: Check that requested observations exist
5. **Validate inputs**: Ensure offsets are within valid ranges
6. **Consider gas costs**: Timestamp queries may be slightly more expensive
7. **Use the Central View Contract**: Always query through the Safe Price View contract, not individual pairs

## References

- Implementation: [src/safe_price.rs](src/safe_price.rs)
- View endpoints: [src/safe_price_view.rs](src/safe_price_view.rs)
- Tests: [tests/pair_rs_test.rs](tests/pair_rs_test.rs)
