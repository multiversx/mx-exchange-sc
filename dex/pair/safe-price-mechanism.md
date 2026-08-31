# Safe Price Integration Guide

This document describes how external applications and smart contracts should consume the xExchange Safe Price mechanism. It covers the public interfaces, time units, query behavior, legacy-observation compatibility, and integration constraints. Deployment and protocol-upgrade procedures are intentionally outside its scope.

## Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Integration Architecture](#integration-architecture)
4. [Time and Observation Model](#time-and-observation-model)
5. [Query Semantics](#query-semantics)
6. [Public Endpoints](#public-endpoints)
7. [LP-Supply Compatibility](#lp-supply-compatibility)
8. [Router Configuration](#router-configuration)
9. [Errors and Edge Cases](#errors-and-edge-cases)
10. [Integration Checklist](#integration-checklist)
11. [Appendix: Direct Storage Reading](#appendix-direct-storage-reading)

## Overview

Safe Price is a time-weighted average price (TWAP) mechanism for xExchange Pairs. It accumulates reserves over elapsed time and uses cumulative differences to calculate token-to-token quotes and LP-token values over a requested interval.

For production use, integrators should validate the Pair address, choose a suitable lookback, handle unavailable history explicitly, and define how Pair pause state affects their own application.

Key properties:

- cumulative weights and positive observation timestamps use milliseconds;
- finalized history is bounded to 65,536 observations per Pair;
- one additional `current_price_observation` stores the latest cumulative state between finalizations;
- observations are updated by Pair operations, not by the passage of time alone;
- timestamp queries are independent of round-duration inference;
- round queries remain available for compatibility, with the limitations described below.

## Quick Start

1. Obtain the canonical Safe Price View and Router addresses for the target network and release.
2. Resolve the Pair through `Router.getPair(first_token_id, second_token_id)` and verify that the Pair's `getRouterManagedAddress`, `getFirstTokenId`, and `getSecondTokenId` values match the expected Router and token identifiers.
3. Choose an endpoint from the table below. New timestamp integrations should use an `Ms` endpoint.
4. Query the Safe Price View, passing the verified Pair address as the first argument.
5. Validate that the returned token identifiers and amounts match the application's expected assets and minimum-value policy.
6. Treat a rejected query as unavailable oracle data and apply the application's explicit reject, defer, pause, or independently secured fallback policy.

Use release metadata that binds each network and shard address to the matching Safe Price View ABI and code version. Treat this metadata as a required integration input and keep the addresses configurable.

For off-chain flows that perform several validation queries, read them from the same finalized block snapshot. A later transaction can observe newer oracle state, so state-changing logic should either query synchronously during execution or enforce its own limits and slippage constraints.

### Which endpoint should I use?

| Integration need | Recommended endpoint |
|---|---|
| Token quote over an explicit millisecond lookback | `getSafePriceByTimestampOffsetMs` |
| LP-token value over an explicit millisecond lookback | `getLpTokensSafePriceByTimestampOffsetMs` |
| Best available history, up to the Router-configured default | `getSafePriceByDefaultOffset` or `getLpTokensSafePriceByDefaultOffset` |
| Existing seconds-based integration | Compatibility endpoint without the `Ms` suffix |
| Existing round-based integration | Round endpoint, subject to the cadence limitations below |
| Custom cumulative-data processing | `getPriceObservation`; see its semantics before use |

## Integration Architecture

### Use the central Safe Price View

New integrations should query the separately deployed `safe-price-view.wasm` contract. Every central query endpoint accepts the target `pair_address` as its first argument. A View deployment can serve compatible Pairs whose storage is accessible from that deployment.

These query endpoints are read-only. Off-chain VM queries do not submit transactions; synchronous smart-contract consumers still pay execution gas.

Because the View reads Pair and Router state synchronously, on-chain integrations must use a compatible View deployment in the Pair's shard. Keep its address configurable by network and release.

The central view:

1. reads the required Safe Price, reserve, LP-supply, token, and Router-address storage from the supplied Pair;
2. for default-window queries, reads the configured lookback from that Pair's Router;
3. normalizes or resolves the requested cumulative boundaries;
4. returns the calculated quote or observation.

Before using a Safe Price result for accounting or other state changes, resolve `pair_address` through the trusted Router registry and confirm that the Pair's `getRouterManagedAddress` value matches the expected Router. This binds the query to the intended Pair and Router-owned configuration.

### Pair pause state

If an integration accepts data only from active Pairs, query the Pair's `getState` view and enforce that requirement before consuming Safe Price. A paused Pair can still return Safe Price data, and permitted operations can continue advancing observations. If the integration requires an active Pair or unchanged observations, check and enforce that policy separately.

### Endpoint availability by artifact

| Artifact | Safe Price interface |
|---|---|
| `safe-price-view.wasm` | Documented central quote and observation endpoints |
| `pair.wasm` | Pair storage views and compatibility quote endpoints |
| `pair-full.wasm` | Pair interfaces plus the central quote and observation endpoints |
| `router.wasm` | Safe Price configuration getters |

Central query endpoints are not exported by a standard `pair.wasm`. Call them on the Safe Price View and pass the Pair address instead.

## Time and Observation Model

### Observation layout

The current observation type has six fields, in this exact order:

```rust
pub struct PriceObservation {
    pub first_token_reserve_accumulated: BigUint,
    pub second_token_reserve_accumulated: BigUint,
    pub weight_accumulated: u64,
    pub recording_round: u64,
    pub recording_timestamp: u64,
    pub lp_supply_accumulated: BigUint,
}
```

For current observations:

- `weight_accumulated` is cumulative elapsed time in milliseconds;
- `recording_timestamp` is a block timestamp in milliseconds;
- reserve and LP-supply accumulators are value-times-milliseconds totals;
- `recording_round` preserves the corresponding blockchain round.

Raw legacy observations use a different four-field encoding. External storage readers must follow [Legacy Observation Normalization](#legacy-observation-normalization) before mixing legacy and current cumulative values.

### Recording behavior

Swap, add-liquidity, and remove-liquidity paths call the Safe Price writer before changing reserves. For an existing observation, a valid update adds:

```text
elapsed_ms = current_timestamp_ms - previous_timestamp_ms

first_reserve_accumulator  += first_reserve  * elapsed_ms
second_reserve_accumulator += second_reserve * elapsed_ms
lp_supply_accumulator      += lp_supply      * elapsed_ms
weight_accumulated         += elapsed_ms
```

Eligible Pair operations update `current_price_observation` when reserves and LP supply are non-zero and the timestamp advances. Every such update replaces it with the latest cumulative state. Once the configured save interval has elapsed, the same state is also finalized in the circular observation buffer.

Finalization is event-driven: elapsed time alone does not write storage. The first eligible Pair operation after the interval is reached performs the finalization. Multiple operations at the same timestamp add no additional weight after the first update.

For a fresh oracle without a previous timestamp, the first contribution uses the runtime-reported round duration. Later updates use timestamp differences, so Pair recording is not tied to a hardcoded current round duration.

### Finalized and current observations

The Pair retains at most 65,536 finalized observations in a circular buffer. `safe_price_current_index` identifies the newest finalized entry. `current_price_observation` is stored separately and is either equal to that finalized observation or newer.

A fresh Pair can therefore have a pending current observation before any history is finalized. Queries reject this state until at least one finalized observation and a current observation exist.

## Query Semantics

### Cumulative calculation

For two resolved cumulative observations, the implementation calculates:

```text
duration_ms = end.weight_accumulated - start.weight_accumulated

weighted_first_reserve =
    (end.first_reserve_accumulator - start.first_reserve_accumulator) / duration_ms

weighted_second_reserve =
    (end.second_reserve_accumulator - start.second_reserve_accumulator) / duration_ms
```

A token quote then uses the ratio of the weighted reserves:

```text
output_amount = input_amount * weighted_output_reserve / weighted_input_reserve
```

All divisions are integer divisions and round down.

Safe Price averages each reserve over the interval and then takes the ratio of those weighted reserves. Treat the result as a reference valuation. An executable swap quote should separately account for swap fees, constant-product price impact, routing, and slippage protection.

### Boundary resolution

The View uses exact stored observations when available, interpolates between valid neighboring observations, and extrapolates from the latest persisted state to the current block. It rejects requests outside retained history or across inconsistent cumulative data. These calculations do not modify storage.

### Timestamp offsets

The `Ms` endpoints are the primary timestamp APIs. Their offsets are milliseconds. They calculate a trailing interval ending at the current block timestamp:

```text
[current_timestamp_ms - offset_ms, current_timestamp_ms]
```

The compatibility endpoints without the `Ms` suffix accept seconds, multiply the value by 1,000 with checked arithmetic, and delegate to the corresponding `Ms` endpoint.

Only the `ByDefaultOffset` endpoints shorten their window to available elapsed history, defined as the span from the oldest valid observation to the current block. Explicit timestamp offsets are not clamped: they are rejected if they begin before retained history. For an `Ms` endpoint, `0 < offset_ms < current_timestamp_ms`. A seconds wrapper first performs its checked multiplication by 1,000, then applies that millisecond condition.

When an integration requires a minimum manipulation-resistance window, use an explicit `Ms` offset so insufficient history fails, or verify the available span independently before accepting a default-window result. Default-window queries may use a shorter effective span when the full configured history is not yet available.

### Round ranges

Round-based endpoints map rounds to timestamps and then use the same timestamp search, interpolation, and extrapolation path.

A round offset must satisfy `0 < round_offset < current_round`. Explicit round ranges require `end_round > start_round`, and every requested round must lie between the normalized oldest anchor and the current round.

The current solver supports a history with either no cadence transition or one transition from the legacy 6,000-millisecond round duration to the current runtime duration. It relies on scheduled round numbers not being skipped and requires the elapsed rounds and elapsed time to satisfy that cadence model exactly. Round-based endpoints are retained for compatibility; timestamp-based `Ms` endpoints are the preferred integration interface.

## Public Endpoints

All central endpoints take `pair_address: Address` as their first argument.

### Token-to-token quotes

| Endpoint | Remaining arguments | Window | Output |
|---|---|---|---|
| `getSafePriceByDefaultOffset` | `input_payment: EsdtTokenPayment` | Router default in milliseconds, shortened to available elapsed history | `EsdtTokenPayment` |
| `getSafePriceByTimestampOffsetMs` | `timestamp_offset_milliseconds: u64`, `input_payment: EsdtTokenPayment` | Trailing milliseconds | `EsdtTokenPayment` |
| `getSafePriceByTimestampOffset` | `timestamp_offset_seconds: u64`, `input_payment: EsdtTokenPayment` | Trailing seconds; compatibility wrapper | `EsdtTokenPayment` |
| `getSafePriceByRoundOffset` | `round_offset: u64`, `input_payment: EsdtTokenPayment` | `[current_round - offset, current_round]` | `EsdtTokenPayment` |
| `getSafePrice` | `start_round: u64`, `end_round: u64`, `input_payment: EsdtTokenPayment` | Explicit round range | `EsdtTokenPayment` |

`input_payment.token_identifier` must be one of the Pair's two tokens. The returned payment contains the other token and the calculated amount.

### LP-token values

| Endpoint | Remaining arguments | Window | Output |
|---|---|---|---|
| `getLpTokensSafePriceByDefaultOffset` | `liquidity: BigUint` | Router default in milliseconds, shortened to available elapsed history | Two `EsdtTokenPayment` values |
| `getLpTokensSafePriceByTimestampOffsetMs` | `timestamp_offset_milliseconds: u64`, `liquidity: BigUint` | Trailing milliseconds | Two `EsdtTokenPayment` values |
| `getLpTokensSafePriceByTimestampOffset` | `timestamp_offset_seconds: u64`, `liquidity: BigUint` | Trailing seconds; compatibility wrapper | Two `EsdtTokenPayment` values |
| `getLpTokensSafePriceByRoundOffset` | `round_offset: u64`, `liquidity: BigUint` | `[current_round - offset, current_round]` | Two `EsdtTokenPayment` values |
| `getLpTokensSafePrice` | `start_round: u64`, `end_round: u64`, `liquidity: BigUint` | Explicit round range | Two `EsdtTokenPayment` values |

The two returned payments represent the first-token and second-token value of the requested LP amount. See [LP-Supply Compatibility](#lp-supply-compatibility) before using an LP result for state-changing accounting.

### Observation query

`getPriceObservation(pair_address: Address, search_round: u64) -> PriceObservation` returns the exact, normalized, interpolated, or extrapolated cumulative observation for a valid round. The returned six-field value has `recording_round` set to the requested round and `recording_timestamp` set to its inferred millisecond timestamp.

This is cumulative oracle state, not a token price or executable quote. Use the quote endpoints unless the integration intentionally implements cumulative-difference calculations. Unlike a quote, this endpoint can request one valid round and does not require a non-zero range between two requested rounds.

### Pair storage views

The following views are available directly on `pair.wasm` and `pair-full.wasm`:

| Endpoint | Output | Meaning |
|---|---|---|
| `getSafePriceCurrentIndex` | `u32` in the WASM ABI | Zero if nothing is finalized; otherwise the 1-based newest finalized index |
| `getCurrentPriceObservation` | `PriceObservation` | Latest persisted cumulative state, finalized or in progress |
| `getSafePriceLegacyCutover` | `(u64, u64)` | Legacy normalization anchor `(round, timestamp_ms)` when the Pair has legacy history |

### Legacy Pair quote endpoints

Standard Pairs retain these endpoints for backwards compatibility:

- `updateAndGetSafePrice(input: EsdtTokenPayment) -> EsdtTokenPayment` delegates to the default token Safe Price query;
- `updateAndGetTokensForGivenPositionWithSafePrice(liquidity: BigUint) -> (EsdtTokenPayment, EsdtTokenPayment)` delegates to the default LP-value query.

Despite their historical names, these endpoints do not update Safe Price storage. New integrations should use the central Safe Price View.

### Argument-order examples

The following pseudocode shows the contract address, endpoint name, and argument order. Encode and decode every argument through the MultiversX SDK using the exact ABI distributed with the matching View release; `query(...)` is not an SDK function.

Assume `pair_address` has passed the validation steps above. For fungible Pair tokens, construct `input_payment` as `EsdtTokenPayment(input_token_id, nonce = 0, positive_amount)`, and provide `lp_amount` as a positive `BigUint`. The token query returns one nonce-zero payment for the opposite Pair token. The LP query returns two nonce-zero payments ordered as the Pair's first token and second token.

```text
query(
    contract = canonical_safe_price_view_address,
    endpoint = "getSafePriceByTimestampOffsetMs",
    arguments = [pair_address, 3_600_000, input_payment],
)
```

This calculates a token TWAP over the latest hour.

```text
query(
    contract = canonical_safe_price_view_address,
    endpoint = "getLpTokensSafePriceByDefaultOffset",
    arguments = [pair_address, lp_amount],
)
```

This returns the first-token and second-token value of `lp_amount` over the effective default window.

## LP-Supply Compatibility

Legacy four-field observations did not record cumulative LP supply, and historical LP supply cannot be reconstructed from their encoding. Their normalized `lp_supply_accumulated` therefore remains zero.

For an LP-value query:

1. the view calculates time-weighted LP supply only when the first resolved observation has a positive LP-supply accumulator;
2. if the calculated weighted LP supply is zero, it falls back to the Pair's current LP supply;
3. if the current LP supply is also zero, it returns two zero-amount payments.

A range beginning in legacy history uses the Pair's current LP supply as its denominator. If LP supply changed during that range, the result is not fully time-weighted.

Define `legacy_lp_boundary_ms` as the normalized timestamp of the newest legacy observation. A default-window LP query is fully time-weighted only when:

```text
current_timestamp_ms - effective_default_offset_ms > legacy_lp_boundary_ms
```

and the first resolved observation has a positive LP-supply accumulator. The inequality is strict: equality still begins at the legacy boundary and uses the fallback.

`effective_default_offset_ms` is the Router-configured default shortened to available elapsed history. If the Router changes the default, re-evaluate the condition.

To accept a fully time-weighted LP result, verify the strict boundary condition above and a positive LP-supply accumulator at the first observation. Obtain the boundary from trusted Pair metadata or normalized raw storage. If the boundary cannot be established, do not classify the result as fully time-weighted. The LP response contains only the two token payments and does not indicate which denominator was used. Token-to-token Safe Price quotes are unaffected by this LP-supply condition.

## Router Configuration

Safe Price configuration is stored in the Router and read directly by Pairs and the central view.

| Getter | Unit | Initialization default | Meaning |
|---|---|---:|---|
| `getSafePriceTimestampSaveInterval() -> u64` | milliseconds | `6,000` | Minimum elapsed time before a writer update can finalize another observation |
| `getDefaultSafePriceTimestampOffset() -> u64` | milliseconds | `3,600,000` | Default trailing query window |

These values are mutable and must be positive. External integrations should read the live Router getters rather than hardcode the initialization defaults, and should treat missing or zero configuration as unavailable.

## Errors and Edge Cases

Queries reject empty or pending-only history, zero or excessive offsets, incorrectly ordered explicit round ranges, targets before retained history or after the current block, overflowing seconds-to-milliseconds conversion, and round history that does not satisfy the supported cadence model.

Only default-offset endpoints shorten their requested window to available elapsed history. Treat any rejected query as unavailable oracle data until the application's retry or fallback policy permits another action.

## Integration Checklist

Before relying on Safe Price:

1. Obtain the canonical Safe Price View and Router addresses for the Pair's network, release, and shard.
2. Resolve the Pair through `Router.getPair`, then verify its managed Router and token identifiers.
3. Use the `Ms` endpoints for new timestamp-based integrations; retain seconds and round endpoints only where compatibility requires them.
4. Query the live Router configuration instead of assuming its initialization defaults.
5. Validate returned token identifiers and apply the application's amount and rounding policy.
6. Define an explicit oracle-unavailable policy that rejects, defers, or pauses the affected action. Use a fallback only when it is independently secured, intentionally configured, and clearly distinguished from Safe Price.
7. Decide whether a paused Pair is acceptable and enforce that policy independently.
8. Enforce the LP-supply compatibility condition before treating a legacy-transition LP result as fully time-weighted.
9. Use a lookback long enough for the application's manipulation and responsiveness requirements.
10. Account for cross-contract storage reads and binary-search execution cost in synchronous on-chain calls.

## Appendix: Direct Storage Reading

This appendix applies only to integrations that decode Pair observation storage themselves. Consumers that use the official Safe Price View receive normalized cumulative observations and calculated quotes from the contract.

Normalization alone does not reproduce an official quote. A custom implementation must also reproduce boundary search, interpolation, extrapolation, Router configuration, and the required current reserves and LP supply, all from a consistent block snapshot.

### Relevant storage

| Storage key | Meaning |
|---|---|
| `price_observations` | `VecMapper` containing finalized raw observations |
| `safe_price_current_index` | 1-based physical index of the newest finalized observation |
| `current_price_observation` | Latest persisted six-field observation |
| `safe_price_legacy_cutover` | Legacy normalization anchor `(round, timestamp_ms)` |

`price_observations` uses the MultiversX framework's `VecMapper` storage encoding; it is not one flat encoded value. Before the buffer is full, physical indices `1..=len` are chronological. At full capacity, the oldest entry is `(safe_price_current_index % 65_536) + 1`; chronological traversal wraps from there through the current index.

Fetch all required keys from the same finalized block. Validate the vector length and current index, then validate normalized timestamp and cumulative-weight ordering, including that `current_price_observation` is not older than the newest finalized observation.

### Supported raw encodings

Exactly two observation layouts are supported:

1. Legacy four-field layout:
   1. `first_token_reserve_accumulated`
   2. `second_token_reserve_accumulated`
   3. `weight_accumulated`
   4. `recording_round`
2. Current six-field layout:
   1. the same four fields;
   2. `recording_timestamp`;
   3. `lp_supply_accumulated`.

Decode only the four-field legacy and six-field current layouts. Treat other encodings as unsupported. Positive stored timestamps are milliseconds and must not be rescaled.

### Legacy Observation Normalization

`recording_timestamp == 0` identifies an observation that requires legacy normalization. A valid legacy observation also has `lp_supply_accumulated == 0`.

For a Pair with legacy finalized history, read its immutable `safe_price_legacy_cutover` value, exposed by `getSafePriceLegacyCutover`:

```text
(cutover_round, cutover_timestamp_ms)
```

This is a Pair-specific normalization anchor, not a network-wide activation value. A Pair created with only current six-field observations does not need a legacy cutover.

The contract normalizes each legacy observation in memory as follows:

```text
require observation.lp_supply_accumulated == 0
require legacy cutover is available
require observation.recording_round <= cutover_round

elapsed_rounds = cutover_round - observation.recording_round
elapsed_ms = elapsed_rounds * 6_000

require elapsed_ms < cutover_timestamp_ms

observation.recording_timestamp = cutover_timestamp_ms - elapsed_ms
observation.first_token_reserve_accumulated  *= 6_000
observation.second_token_reserve_accumulated *= 6_000
observation.weight_accumulated               *= 6_000
observation.lp_supply_accumulated             = 0
```

The 6,000 multiplier converts legacy round-weighted cumulative values to the canonical millisecond timeline. Raw legacy vector entries are not rewritten.

External readers should additionally validate that `cutover_round` and `cutover_timestamp_ms` are positive and should use checked arithmetic or sufficiently wide integer representations for multiplication, scaling, and subtraction. Treat missing or invalid cutover data, invalid ordering, or invalid arithmetic as unavailable history.

Normalize every legacy boundary before combining it with millisecond-weighted current accumulators.

### Boundary reconstruction

When interpolating between adjacent normalized observations, require ordered timestamps and:

```text
right.weight_accumulated - left.weight_accumulated
    == right.recording_timestamp - left.recording_timestamp
```

The View rejects inconsistent cumulative boundaries. A target after `current_price_observation`, but not after the current block timestamp, is extrapolated in memory using current Pair reserves and LP supply. This reconstruction does not modify Pair storage.

## Source References

- Pair recording and normalization: [src/safe_price.rs](src/safe_price.rs)
- Central view endpoints and query logic: [src/safe_price_view.rs](src/safe_price_view.rs)
- Cross-contract storage keys: [src/read_pair_storage.rs](src/read_pair_storage.rs)
- Artifact endpoint selection: [sc-config.toml](sc-config.toml)
