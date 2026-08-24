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

The Safe Price mechanism is a Time-Weighted Average Price (TWAP) oracle implementation designed to provide manipulation-resistant price data for DEX pairs. It accumulates Pair reserves over elapsed time and finalizes a cumulative observation on the first eligible Pair operation after the configured interval elapses. A sufficiently long lookback reduces the influence of short-lived price manipulation, but does not make every integration safe by itself.

### Key Benefits

- **Manipulation Resistance**: TWAP calculations reduce the influence of short-lived price manipulation
- **Historical Data**: Maintains up to 65,536 price observations
- **Flexible Queries**: Supports both round-based and timestamp-based price lookups
- **Gas Efficiency**: Configurable recording intervals optimize gas costs
- **Millisecond Timeline**: Preserves elapsed-time weighting across the planned `6,000` to `600` millisecond round-duration transition

## Architecture and Deployment

### Central Safe Price View Contract

**IMPORTANT**: dApps should query safe price data through the **Central Safe Price View Contract**, not individual pair contracts. This is the only recommended approach for production use.

#### Why Use the Central View Contract

- **Unified Interface**: Single contract address for all safe price queries across all pairs
- **Pair Address as Parameter**: Query any pair by passing its address as a parameter
- **View-Only Operations**: The central contract exposes read-only views. Off-chain queries do not submit transactions; synchronous on-chain consumers still pay execution gas
- **Stable Integration**: Contract address remains constant even as new pairs are added
- **Cross-Pair Queries**: Easily query multiple pairs without managing multiple contract addresses

#### How It Works

The Safe Price View contract reads price observation data directly from individual pair contract storage. When you call an endpoint with a `pair_address` parameter, it:

1. Reads the pair's safe price observations from storage
2. Performs all calculations (interpolation, weighted averaging)
3. Returns manipulation-resistant price data

All historical data remains stored in the pair contracts, while the view contract provides a centralized query interface. The central view accepts an arbitrary `pair_address` and does not authenticate it against the Router registry, so state-changing consumers must use a trusted Pair address.

#### Deployment

The Safe Price View contract is built as a separate WASM module (`wasm-safe-price-view`) that exposes view endpoints from the pair contract's `SafePriceViewModule`.

**Build artifact**: `safe-price-view.wasm`. Operators must separately record and verify the deployed contract address and code hash.

#### Supernova Upgrade Order

The migration must be executed in this order:

1. Rebuild the Router, Pair, Pair Full, and central Safe Price View WASM artifacts reproducibly. Record and verify every exact code hash before deployment.
2. Pause every Pair and every state-changing Safe Price consumer, including Farm Staking Proxy flows. Stop or temporarily revoke every whitelisted maintenance caller that can invoke a liquidity endpoint while a Pair is paused, including the buyback-and-burn flow.
3. Upgrade the Router, verify that both Safe Price configuration values are present in milliseconds and that `temporary_owner_period` is `30` seconds, and resume only the Router. Do not perform the upgrade while a newly created Pair is still using temporary-owner permissions to issue its LP token.
4. Deploy or upgrade the standard Pair template from the verified `pair.wasm`, update the Router template address if necessary, and verify its code hash.
5. Upgrade every standard Pair before Supernova activation while runtime round duration is still `6,000` milliseconds. Router-triggered Pair upgrades are asynchronous and have no callback, so verify every Pair individually. For each Pair with finalized history, verify a positive `(round, timestamp_ms)` cutover through `getSafePriceLegacyCutover` and a non-empty normalized `current_price_observation`. Empty-history Pairs intentionally keep both values empty at upgrade.
6. Upgrade any deployed `pair-full.wasm` instance from the separately verified `pair-full.wasm` source. Do not copy the standard Pair template onto a Pair Full instance, because that would remove its view endpoints. If the Router template is temporarily changed for this operation, restore and re-verify the standard template afterward.
7. Only after every Pair has been upgraded, upgrade the separate `safe-price-view.wasm` contract and verify legacy, cross-transition, and current-history queries.
8. Resume the Pairs and suspended maintenance callers only after the central view is compatible with six-field observations.
9. As a rollout safety policy for state-changing consumers that require a fully time-weighted LP supply, keep those consumers disabled until the start of their default lookback is strictly later than the LP legacy boundary. The exact condition is `current_timestamp_ms - effective_default_offset_ms > legacy_lp_boundary_ms`. Verify that the selected first observation has a positive `lp_supply_accumulated` before enabling Farm Staking Proxy flows. The Pair does not enforce this gate itself; before maturity, it remains callable and uses the legacy current-supply fallback.

Pair pause alone is not a complete write freeze: the whitelisted buyback-and-burn liquidity path can update and finalize Safe Price observations while paused. That actor must remain stopped during the mixed-binary window so the old central view never encounters a newly finalized six-field observation. Pair pause also does not disable Safe Price reads, so dependent contracts must be gated separately when required.

#### Legacy Endpoints

Individual pair contracts still expose safe price endpoints (`updateAndGetSafePrice`, `updateAndGetTokensForGivenPositionWithSafePrice`) for backwards compatibility only. These are **not recommended** for new integrations.

### Important Security Consideration

**CRITICAL**: The Safe Price module retrieves data independently of the liquidity pool's active/paused state. Even if a Pair is paused, the Safe Price module continues to return data.

**For external contract integrations**: If your contract requires awareness of the pair's operational status, you must **manually check the liquidity pool's pause state** before using safe price data. The safe price mechanism does not enforce or reflect pause states.

This design allows price queries to remain available for informational purposes while giving integrating contracts full control over how they handle paused pool scenarios. Integrations must also validate that the queried address is an approved Pair because the central view does not perform that registry check.

## Protocol Upgrade: Time-Based Safe Price

### Background

With the MultiversX protocol upgrade reducing round duration from 6 seconds to 0.6 seconds, Safe Price accumulation moved to a canonical millisecond timeline instead of using round counts as time weights.

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

#### 2. Millisecond Timestamp Endpoints and ABI Compatibility

- `getSafePriceByTimestampOffsetMs`: Get safe price using a millisecond timestamp offset
- `getLpTokensSafePriceByTimestampOffsetMs`: Get LP token value using a millisecond timestamp offset

These are the primary timestamp-offset endpoints. They query elapsed time in **milliseconds** rather than rounds. Every positive `recording_timestamp`, save interval, default offset, and offset passed to an `Ms` endpoint is expressed in milliseconds.

The deployed endpoint names remain available with their original seconds-based ABI:

- `getSafePriceByTimestampOffset`
- `getLpTokensSafePriceByTimestampOffset`

Each compatibility endpoint multiplies its seconds argument by `1,000` and delegates to the corresponding `Ms` endpoint. Existing callers continue passing `3,600` for one hour; new callers should use the `Ms` endpoint and pass `3,600,000`. The `ByDefaultOffset` endpoints read the Router-owned millisecond default and call the `Ms` implementation directly.

This ABI compatibility is separate from observation migration: no timestamp-bearing observation format with positive seconds was deployed. Consumers of `getPriceObservation` must still decode the new six-field return value.

#### 3. Round-to-Timestamp Compatibility

Round-based endpoints remain available for compatibility, but round numbers are no longer a separate lookup axis. Under the protocol's deterministic round schedule, the view normalizes the oldest retained observation to a positive millisecond timestamp and combines that anchor with the current round, current timestamp, and runtime round duration. Assuming the single protocol transition from the legacy `6,000` millisecond cadence to the current `600` millisecond cadence, it solves the exact number of legacy-duration rounds and infers the requested round's timestamp.

This relies on the protocol invariant that round numbers represent every scheduled round and are not skipped like produced block nonces. The elapsed round count and elapsed timestamp must therefore satisfy the supported cadence equation exactly.

The inferred timestamp is then resolved through the same timestamp binary-search and interpolation path used by timestamp-based endpoints. Inconsistent timelines and rounds outside the normalized anchor-to-current range are rejected. The solver is exact for the supported zero- or one-transition history, but it does not encode a complete cadence history and is not safe for a later third cadence. Before any future round-duration change, round-based endpoints must be deprecated or extended with explicit cadence-transition data. Timestamp-based endpoints remain the preferred interface.

#### 4. Intermediate Save Functionality

To optimize gas costs with faster block times, the system now supports:

- **Configurable Save Intervals**: Set how often observations are finalized
- **Current Accumulation**: Keeps the latest cumulative state available between saves
- **Event-Driven Finalization**: Saves a cumulative observation on the first eligible Pair operation after the interval is reached

#### 5. Legacy Cutover and Normalization

Every pair with legacy history must be upgraded before protocol activation. Pair upgrade stores an immutable `(round, timestamp_ms)` normalization anchor while the legacy cadence is still active. This pair-upgrade anchor is not the protocol activation round or timestamp.

Raw legacy observations are identified only by `recording_timestamp == 0`; every positive stored timestamp is already milliseconds and is never interpreted as seconds.

- A legacy observation timestamp is inferred only from the immutable cutover: `cutover_timestamp_ms - (cutover_round - observation_round) * 6,000`.
- A pair with legacy history and a missing or invalid cutover fails closed.
- Values written to `current_price_observation` are created only by the upgraded binary, always contain a positive millisecond timestamp, and never participate in legacy normalization.
- Legacy accumulators and weights are multiplied by `6,000` only after a valid positive timestamp is inferred. A truly empty oracle at chain timestamp zero is the separate no-write bootstrap case.
- Pair upgrade initializes `current_price_observation` from the latest finalized observation. A legacy latest observation is normalized in memory before it is stored in the current mapper; the legacy vector entry remains unchanged.
- A Pair with no finalized Safe Price observation returns early from this migration: it needs neither a legacy cutover nor legacy normalization. Its first valid update starts a native six-field millisecond observation.

The supported runtime cases are:

| Case | Runtime round duration | Stored timestamp | Handling |
|---|---:|---:|---|
| Legacy observation read before or after activation | `6,000` or `600` ms | `0` | Infer timestamp from the pre-activation Pair cutover and scale round-weighted cumulative values by `6,000` |
| Upgraded Pair before Supernova | `6,000` ms | Positive milliseconds | Use unchanged; accumulate with millisecond deltas |
| Upgraded Pair after Supernova | `600` ms | Positive milliseconds | Use unchanged; accumulate with millisecond deltas |

The `600` millisecond cadence affects fresh-oracle bootstrap weight and round-to-timestamp compatibility. It does not trigger another normalization of observations that already have a positive timestamp.

#### LP-Supply Migration Boundary

Legacy four-field observations contain no LP-supply accumulator. Timestamp normalization converts their reserve accumulators and weights to milliseconds, but `lp_supply_accumulated` remains zero because historical LP supply cannot be reconstructed from the deployed schema.

For an LP-value query, the implementation uses time-weighted LP supply only when the first resolved observation has `lp_supply_accumulated > 0`. If it is zero, the query preserves legacy behavior by dividing the time-weighted reserves by the Pair's current LP supply. This fallback is decoding- and storage-compatible, but it is not a time-consistent denominator if LP supply changes around the queried period.

Define `legacy_lp_boundary_ms` as the normalized timestamp of the newest legacy observation. On upgrade, this is the timestamp of the observation used to initialize `current_price_observation`. A Pair is mature for default LP-value queries only when:

```text
current_timestamp_ms - effective_default_offset_ms > legacy_lp_boundary_ms
```

and the first resolved observation has a positive LP-supply accumulator. The inequality is strict: equality still selects the zero-accumulator boundary observation and activates the fallback. `effective_default_offset_ms` is the Router-configured default offset, shortened to the available history when necessary.

For a rollout in which state-changing accounting consumers require a fully time-weighted LP supply, keep consumers of `getLpTokensSafePriceByDefaultOffset` and `updateAndGetTokensForGivenPositionWithSafePrice`, including Farm Staking Proxy flows, disabled until this condition is verified independently for every Pair. This is an integration policy, not a Pair-level execution guard: before maturity, the endpoints remain callable and use the current-supply fallback. The boundary can mature while a Pair remains paused when reserves and supply are unchanged because views extrapolate the current state in memory. If the Router default offset changes, maturity must be re-evaluated. Explicit LP-value ranges whose first resolved observation is legacy continue using the current-supply fallback even after the default range has matured. Token-to-token Safe Price queries do not use LP supply and are not subject to this specific limitation.

## How It Works

### Recording Process

1. **Before Each Reserve Change**: Swap, add-liquidity, and remove-liquidity paths pass the current pre-mutation reserves and LP supply to the writer
2. **Elapsed-Time Accumulation**: The writer extends the cumulative observation from its last timestamp to the current block timestamp
3. **Interval Check**: Once finalized history exists, the writer compares elapsed milliseconds with the newest finalized observation; during bootstrap, it uses the current observation's cumulative weight. When the threshold is reached, the cumulative value is also written to the circular buffer
4. **Current Observation**: `current_price_observation` is updated after every valid distinct-timestamp write, whether or not finalization occurs
5. **Circular Buffer**: The most recent finalized observations are retained up to `MAX_OBSERVATIONS`

The mechanism is event-driven: elapsed time alone does not create a storage write. Finalization happens on the next eligible Pair operation. Multiple operations at the same block timestamp do not add weight twice; after the first update, later same-timestamp calls are no-ops for Safe Price accumulation.

### Calculation Method

Safe price uses time-weighted averaging:

```
Weighted Reserve = (Σ reserve_i × time_i) / (Σ time_i)

Weighted LP Supply = (Σ lp_supply_i × time_i) / (Σ time_i)
```

Where:
- `reserve_i` is the token reserve at observation i
- `lp_supply_i` is the LP supply at observation i when the queried range has native LP-supply history
- `time_i` is the duration (weight) of that observation
- The sum covers all observations in the specified time range

### Price Query Process

1. **Determine Time Range**: Use the configured/default timestamp offset, an explicit timestamp offset, or explicit round boundaries
2. **Resolve Boundaries**: Find exact observations, interpolate between available cumulative boundaries (whose right boundary may be the non-finalized `current_price_observation`), or extrapolate the latest observation with the Pair's current reserves and LP supply
3. **Calculate Weighted Amounts**: Subtract cumulative values and divide by elapsed millisecond weight
4. **Return Price**: Calculate the token ratio or LP-token value from the weighted amounts

Default-offset queries use the Router-configured lookback, shortened to the amount of history available. They still require a positive time range and at least one finalized observation plus a current observation.

## Price Observation Structure

### Circular Buffer Storage

Price observations are stored in a **circular buffer** (circular list) with a maximum capacity of **65,536 observations** (2^16). This data structure provides:

- **Efficient Storage**: Automatically overwrites oldest data when capacity is reached
- **Fast Lookups**: Optimized for binary search operations
- **Predictable Memory**: Fixed maximum storage footprint
- **Rolling Window**: Always maintains the most recent observation history

The circular buffer provides bounded finalized history. Recording is event-driven rather than continuous: eligible Pair operations update storage, while views can extend the latest cumulative observation in memory to a requested timestamp without persisting that extension.

`safe_price_current_index` points to the newest finalized vector entry. When the vector is full, the oldest entry is `(current_index % MAX_OBSERVATIONS) + 1`, and lookup searches the correct physical segment around the wrap. `current_price_observation` is stored separately and represents the same or a newer cumulative state than the newest finalized entry.

### Observation Fields

Each price observation records:

- **first_token_reserve_accumulated**: Cumulative weighted first token reserve
- **second_token_reserve_accumulated**: Cumulative weighted second token reserve
- **weight_accumulated**: Cumulative time weight in milliseconds
- **recording_round**: Blockchain round when recorded
- **recording_timestamp**: Block timestamp in milliseconds when recorded
- **lp_supply_accumulated**: Cumulative weighted LP token supply

These fields describe upgraded observations and normalized in-memory legacy observations. Raw legacy vector entries remain in their deployed four-field, round-weighted encoding; upgrade does not rewrite them in place. Views normalize raw legacy entries when reading them.

### Weight Calculation

`weight_accumulated` is the cumulative sum of every elapsed-time contribution. For an existing observation, one writer update adds:

**Incremental Weight = Current Timestamp Milliseconds - Observation Timestamp Milliseconds**

For a fresh oracle without a previous timestamp, the first contribution uses the runtime round duration: `6,000` milliseconds before Supernova or `600` milliseconds after activation. If 600 milliseconds elapsed after an existing observation, the writer adds 600 to `weight_accumulated` and adds `reserve × 600` to each reserve accumulator.

Views calculate an interval's duration by subtracting the two cumulative weights. This gives longer-lived states proportionally greater influence and reduces the effect of rapid reserve changes.

## Recording Mechanisms

### Default Finalization Interval (6,000 Milliseconds)

The default interval retains the historical six-second observation cadence:

```rust
safe_price_timestamp_save_interval = 6_000 // milliseconds (default)
```

- Before the first finalized observation exists, the millisecond-weighted cumulative duration is used as the interval clock.
- A fresh oracle uses the protocol's runtime round duration for its first weight: `6,000` milliseconds before Supernova and `600` milliseconds after activation.
- Later updates within the same 6,000-millisecond window replace the single current observation.
- On the first eligible update where elapsed time since the last finalized observation has reached 6,000 milliseconds, the current state is also written to the circular buffer. It remains in `current_price_observation` as the canonical latest state.

### Configurable Intermediate Save Mode

Any positive configured interval controls how long millisecond-weighted data accumulates before finalization. For example:

```rust
safe_price_timestamp_save_interval = 60_000 // milliseconds
```

- Keeps exactly one latest value in `current_price_observation` after the first valid update
- Finalizes the observation on the next eligible update after the interval in milliseconds has passed
- Retains the finalized value in `current_price_observation`; later updates replace it with the next cumulative value
- Longer intervals lower circular-buffer and index-write frequency but delay finalized history

## Available Endpoints

### Artifact Availability

| Artifact | Safe Price endpoints |
|---|---|
| `pair.wasm` | `getSafePriceCurrentIndex`, `getCurrentPriceObservation`, `getSafePriceLegacyCutover`, `updateAndGetTokensForGivenPositionWithSafePrice`, `updateAndGetSafePrice` |
| `safe-price-view.wasm` | The eleven central query endpoints documented below, including both seconds compatibility wrappers, both `Ms` endpoints, and `getPriceObservation` |
| `pair-full.wasm` | Standard Pair endpoints plus all eleven central query endpoints |
| `router.wasm` | The two timestamp configuration setters and two timestamp configuration getters |

The eleven labeled central query endpoints are not exported by a standard `pair.wasm`. Calling, for example, `getSafePriceByTimestampOffsetMs` directly on a standard Pair returns `endpoint not found`; use the central Safe Price View address and pass the Pair address as an argument.

All quote endpoints require at least one finalized vector observation and a non-empty `current_price_observation`. A fresh Pair with only a pending current observation is not queryable yet.

### Token Swap Price Endpoints

#### `getSafePriceByDefaultOffset`

Compute the token-to-token TWAP over the trailing default window ending at the current block timestamp. If positive available history is shorter than the Router-configured default, the effective offset is reduced to the available span.

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

Compute the token-to-token TWAP over the trailing round window `[current_round - round_offset, current_round]`.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `round_offset: Round` - Number of rounds to look back
- `input_payment: EsdtTokenPayment` - Input token and amount

**Returns:**
- `EsdtTokenPayment` - Output token and amount

**Example:**
```rust
// Get the TWAP over the latest 600 rounds
let output = getSafePriceByRoundOffset(pair_addr, 600, input);
```

#### `getSafePriceByTimestampOffsetMs`

Compute the token-to-token TWAP over the trailing millisecond window `[current_timestamp_ms - timestamp_offset, current_timestamp_ms]`. This is not a point-in-time price at the start of the window.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `timestamp_offset_milliseconds: Timestamp` - Number of milliseconds to look back
- `input_payment: EsdtTokenPayment` - Input token and amount

**Returns:**
- `EsdtTokenPayment` - Output token and amount

**Example:**
```rust
// Get the TWAP over the latest hour (3,600,000 milliseconds)
let output = getSafePriceByTimestampOffsetMs(pair_addr, 3_600_000, input);
```

**Note:** This endpoint is time-independent and works across block duration changes.

#### `getSafePriceByTimestampOffset`

Seconds-based compatibility wrapper for the endpoint deployed before this migration. It accepts `timestamp_offset_seconds`, multiplies it by `1,000`, and delegates to `getSafePriceByTimestampOffsetMs`. New integrations should use the `Ms` endpoint directly.

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

Compute the LP-token value over the trailing default window. If positive available history is shorter than the Router-configured default, the effective offset is reduced to the available span.

The fully time-weighted LP-supply guarantee applies only when the first resolved observation has positive `lp_supply_accumulated`. Otherwise the query uses the current-supply legacy fallback described in [LP-Supply Migration Boundary](#lp-supply-migration-boundary).

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

Compute the LP-token value over a trailing round window. If its first resolved observation is legacy, that query continues using the current-supply fallback regardless of whether the default window has already matured.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `round_offset: Round` - Number of rounds to look back
- `liquidity: BigUint` - Amount of LP tokens

**Returns:**
- `MultiValue2<EsdtTokenPayment, EsdtTokenPayment>` - First and second token amounts

#### `getLpTokensSafePriceByTimestampOffsetMs`

Compute the LP-token value over a trailing millisecond window. This is an interval valuation, not a point-in-time valuation at the start timestamp.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `timestamp_offset_milliseconds: Timestamp` - Number of milliseconds to look back
- `liquidity: BigUint` - Amount of LP tokens

**Returns:**
- `MultiValue2<EsdtTokenPayment, EsdtTokenPayment>` - First and second token amounts

**Example:**
```rust
// Get LP value over the latest 30 minutes (1,800,000 milliseconds)
let (token1, token2) = getLpTokensSafePriceByTimestampOffsetMs(pair_addr, 1_800_000, lp_amount);
```

#### `getLpTokensSafePriceByTimestampOffset`

Seconds-based compatibility wrapper for the endpoint deployed before this migration. It accepts `timestamp_offset_seconds`, multiplies it by `1,000`, and delegates to `getLpTokensSafePriceByTimestampOffsetMs`. New integrations should use the `Ms` endpoint directly.

#### `getLpTokensSafePrice`

Get LP token value for a custom round range. A range that begins from a legacy observation uses the current-supply fallback for that query.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `start_round: Round` - Starting round
- `end_round: Round` - Ending round
- `liquidity: BigUint` - Amount of LP tokens

**Returns:**
- `MultiValue2<EsdtTokenPayment, EsdtTokenPayment>` - First and second token amounts

### Observation Query Endpoints

#### `getPriceObservation`

Get the exact, normalized, interpolated, or extrapolated cumulative observation corresponding to a requested round after mapping that round to a supported timestamp.

**Parameters:**
- `pair_address: ManagedAddress` - The pair contract address
- `search_round: Round` - The round to query

**Returns:**
- `PriceObservation` - The observation data (may be interpolated)

### Legacy Endpoints

#### `updateAndGetTokensForGivenPositionWithSafePrice`

Legacy endpoint that calls `getLpTokensSafePriceByDefaultOffset` on the pair itself.

Despite its historical name, this endpoint does not call `update_safe_price` and does not mutate Safe Price storage. Its LP-supply guarantees are subject to [LP-Supply Migration Boundary](#lp-supply-migration-boundary).

#### `updateAndGetSafePrice`

Legacy endpoint that calls `getSafePriceByDefaultOffset` on the pair itself.

Despite its historical name, this endpoint does not call `update_safe_price` and does not mutate Safe Price storage.

### View Endpoints

#### `getSafePriceCurrentIndex`

Returns `0` while no observation has been finalized; otherwise returns the 1-based index of the newest finalized circular-buffer entry. A pending `current_price_observation` can exist while this index is still zero.

**Available on:** Pair contract

**Returns:**
- `usize` in Rust (`u32` in the WASM ABI) - `0` sentinel or a 1-based current index

#### `getSafePriceTimestampSaveInterval`

Returns the configured finalized-observation interval in milliseconds.

**Available on:** Router contract (storage is in router; pair reads from router's storage)

**Returns:**
- `u64` - Number of milliseconds between finalized saves

**Default Value:** `6,000` milliseconds

#### `getCurrentPriceObservation`

Returns the latest recorded observation. After a finalize operation, this is the same cumulative state as the newest circular-buffer entry; between finalizations, it is the newer in-progress state.

For an upgraded Pair with legacy finalized history, this mapper is initialized during upgrade. For a Pair with no finalized history, it remains empty until the first valid writer update.

**Available on:** Pair contract

**Returns:**
- `PriceObservation` - The latest recorded observation, finalized or in progress

#### `getSafePriceLegacyCutover`

Returns the Pair upgrade anchor used to normalize legacy four-field observations and infer their millisecond timestamps.

**Available on:** Pair contract

**Returns:**
- `(Round, Timestamp)` - Pair-upgrade round and timestamp in milliseconds for a Pair with finalized legacy history

#### `getDefaultSafePriceTimestampOffset`

Returns the default timestamp offset for safe price queries.

**Available on:** Router contract (storage is in router; pair reads from router's storage)

**Returns:**
- `u64` - Default offset in milliseconds

**Default Value:** `3,600,000` milliseconds (1 hour)

## Configuration

### Owner-Only Configuration Endpoints

The Router SC acts as the central hub for safe price configuration. Configuration values are stored in the router's storage, and pair contracts read these values directly from the router using external storage reads.

#### `setSafePriceTimestampSaveInterval`

Set how frequently observations are saved.

**Available on:** Router contract

**Storage:** Router contract (pairs read from router's storage via `new_from_address`)

**Parameters:**
- `new_interval_milliseconds: u64` - Milliseconds; must be > 0

**Example:**
- `interval = 6_000` becomes eligible for finalization after six seconds and is finalized by the next valid writer update

#### `setDefaultSafePriceTimestampOffset`

Set the default lookback period for safe price queries.

**Available on:** Router contract

**Storage:** Router contract (pairs read from router's storage via `new_from_address`)

**Parameters:**
- `new_offset_milliseconds: u64` - Milliseconds; must be > 0

**Default Value:**
- `3,600,000` milliseconds (1 hour)

**Note:** Explicit timestamp-offset endpoints still require an offset parameter. The `Ms` endpoints use milliseconds; the deployed compatibility names use seconds. The `ByDefaultOffset` endpoints read this Router-owned millisecond default.

### Router Upgrade and Storage Compatibility

Router `init` and `upgrade` seed the two millisecond configuration keys with `set_if_empty`, preserving any valid values configured before the call. Router upgrade also overwrites the legacy block-count `temporary_owner_period` with `30` seconds and makes the Router inactive; it must be resumed before invoking `upgradePair`.

Legacy `pair_temporary_owner` records store a block nonce where the upgraded type expects a timestamp. Such old values compare as timestamps far in the past and are removed as expired when accessed, so they fail closed rather than extending authority. The migration must not be executed during an active Pair-creation/LP-token-issuance flow because that temporary permission would be invalidated.

Every upgraded Pair writer reads `safe_price_timestamp_save_interval` directly from its Router. A missing or zero value causes an eligible Pair operation that reaches Safe Price accumulation to revert, which is why Router-first deployment is mandatory.

The following round-named keys and ABI endpoints existed only in the unreleased RC implementation and are neither read nor migrated:

- `safe_price_round_save_interval` / `setSafePriceRoundSaveInterval` / `getSafePriceRoundSaveInterval`
- `default_safe_price_rounds_offset` / `setDefaultSafePriceRoundsOffset` / `getDefaultSafePriceRoundsOffset`

### Constants

- **MAX_OBSERVATIONS**: 65,536 (2^16 records for optimized binary search)
- **DEFAULT_SAFE_PRICE_TIMESTAMP_SAVE_INTERVAL_MILLISECONDS**: 6,000
- **DEFAULT_SAFE_PRICE_TIMESTAMP_OFFSET_MILLISECONDS**: 3,600,000

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

### Example 2: Get a Timestamp-Window TWAP

```rust
// Get the TWAP over the latest 30 minutes using a millisecond timestamp offset
let thirty_minutes_milliseconds = 30 * 60 * 1_000;
let output = self.get_safe_price_by_timestamp_offset_ms(
    pair_address,
    thirty_minutes_milliseconds,
    input
);
```

### Example 3: Get LP Token Value

```rust
// Calculate LP-token value over the latest hour
let one_hour_milliseconds = 60 * 60 * 1_000;
let lp_amount = BigUint::from(1000000u64);

let (first_token, second_token) = self.get_lp_tokens_safe_price_by_timestamp_offset_ms(
    pair_address,
    one_hour_milliseconds,
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

The mechanism uses one timestamp-based binary search with `O(log n)` observation lookup. Timestamp endpoints use it directly; round endpoints first perform the exact round-to-millisecond conversion described above and then use the same search. Circular-buffer indexing handles wraparound when the buffer is full.

### Linear Interpolation

When exact observation matches aren't found, the system interpolates:

```rust
// Weighted average calculation
weighted_value = (left_value × left_weight + right_value × right_weight) / total_weight

// Where:
// left_weight = distance to right observation
// right_weight = distance to left observation
```

This produces a deterministic cumulative estimate between neighboring available cumulative boundaries. The right boundary can be the non-finalized `current_price_observation`. The estimate is exact when the cumulative path is linear over that interval; it cannot reconstruct reserve changes whose intermediate states were never recorded.

### Timestamp Search

All Safe Price quote and observation lookups are resolved on the millisecond timeline. A target between available cumulative boundaries is interpolated; the right boundary may be the non-finalized `current_price_observation`. A target after `current_price_observation`, but not after the current block timestamp, is extrapolated in memory using the Pair's current reserves and LP supply; this extension is not persisted. For `getPriceObservation`, the returned observation's `recording_round` and `recording_timestamp` metadata are set to the requested round and its inferred timestamp under the supported cadence model.

### Migration Compatibility

The system supports exactly two raw storage encodings and timestamp cases:

- A raw observation with `recording_timestamp == 0` requires legacy normalization. Its reserve accumulators and weight are round-weighted, so each read normalizes them in memory with the legacy 6,000 millisecond round duration and infers its timestamp from the Pair-upgrade cutover. The raw vector entry is not rewritten.
- An observation emitted by the upgraded writer has `recording_timestamp > 0`. Its timestamp and cumulative fields are already millisecond-weighted and are returned unchanged. A normalized in-memory legacy observation also has a positive timestamp and is intentionally unchanged by later normalization calls.

The custom decoder accepts exactly two storage layouts, in this field order:

1. Deployed legacy four-field observation: first-token reserve accumulator, second-token reserve accumulator, weight, `recording_round`.
2. Upgraded six-field observation: the same four fields, then `recording_timestamp`, then `lp_supply_accumulated`.

The current writer always emits the six-field layout with a positive millisecond timestamp. Structurally, the decoder can also read a six-field value whose timestamp is zero; semantic normalization treats it as legacy only when its LP-supply accumulator is also zero. There is no seconds-based timestamp migration format. Truncated or unexpected encodings are rejected. For a decoded four-field observation, `recording_timestamp` and `lp_supply_accumulated` default to zero until normalization.

### Gas Optimization

The intermediate save functionality reduces circular buffer writes:

- **At the finalize interval**: The observation is written to the circular buffer (`VecMapper`) and also retained in `current_price_observation` (`SingleValueMapper`).
- **Below the finalize interval**: Only `current_price_observation` is replaced; the circular buffer is unchanged.
- **No-write cases**: Updates with zero reserves or LP supply, round zero, timestamp zero, or a timestamp not newer than the current observation return without writing.
- **Trade-off**: Every valid strictly newer-timestamp update writes the current mapper; finalizing updates additionally write the vector and index.
- **Read cost**: Lookups use `O(log n)` search and cross-contract storage reads. Default-offset endpoints currently load observation context once to determine available history and again to execute the range query.

### Price Manipulation Resistance and Limits

Reserve TWAP reduces sensitivity to short-lived reserve movement:

1. **Time-weighted**: A zero-duration reserve change receives no immediate weight; its influence grows only while that state persists
2. **Historical lookback**: Uses data from before potential attack
3. **Configurable periods**: Longer periods generally reduce short-lived influence but react more slowly to genuine market changes
4. **Integration-dependent safety**: Consumers must validate Pair identity, available history, pause-state policy, and the chosen range

Token-to-token quotes use only weighted reserves. LP-token valuation is fully time-consistent only when the first resolved observation has positive LP-supply accumulation. Legacy-start LP ranges use the current-supply fallback and must be handled according to [LP-Supply Migration Boundary](#lp-supply-migration-boundary).

### Edge Cases Handled

- **Empty history**: Quote views reject a Pair without a finalized vector observation or without `current_price_observation`
- **Zero-length ranges and offsets**: Rejected; `getPriceObservation` can still request a single valid round
- **Out of range**: Requested timestamps or rounds outside retained/supported history are rejected
- **Zero reserves or LP supply**: Writer updates are skipped
- **Same timestamp**: A timestamp that is not strictly newer than the current observation does not add weight or write storage
- **Buffer wraparound**: Circular buffer logic maintains correct ordering

## Best Practices for Developers

1. **Validate Pair identity**: Resolve or verify Pair addresses through the trusted Router registry before relying on central-view results.
2. **Check Pair pause state**: If your contract must respect operational state, check it explicitly; Safe Price reads remain available while the Pair is paused.
3. **Use timestamp offsets**: Timestamp endpoints are independent of round-duration inference and are preferred over round endpoints.
4. **Choose an appropriate lookback**: Longer periods generally reduce short-lived influence but are less reactive.
5. **Enforce LP maturity**: Do not use a legacy-start LP range as a fully time-weighted LP valuation.
6. **Handle unavailable history**: Quote views can reject empty, pending-only, out-of-range, or inconsistent history.
7. **Account for on-chain gas**: Cross-contract storage reads and binary search consume gas in synchronous contract calls.
8. **Use the correct artifact**: New integrations should query the central Safe Price View contract; standard Pair contracts expose only the compatibility endpoints listed above.

## References

- Implementation: [src/safe_price.rs](src/safe_price.rs)
- View endpoints: [src/safe_price_view.rs](src/safe_price_view.rs)
- General Pair tests: [tests/pair_rs_test.rs](tests/pair_rs_test.rs)
- Focused timestamp/migration tests: [tests/safe_price_timestamp_canonical_test.rs](tests/safe_price_timestamp_canonical_test.rs)
