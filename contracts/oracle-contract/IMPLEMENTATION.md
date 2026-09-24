# Oracle Contract Implementation

## Overview
Transformed the 23-line oracle stub into a complete price feed submission system with replay protection, staleness validation, and multi-submitter support.

## Architecture

### Module Structure (following aid-contract pattern)

- **types.rs**: Core data structures
  - `PriceSubmission`: Single price submission with replay protection nonce
  - `FeedLatest`: Latest price state for a feed
  - `SubmitterInfo`: Submitter registration metadata

- **errors.rs**: Oracle-specific error codes (500-509)
  - SubmitterNotAuthorized, DuplicateSubmission, FeedNotFound, SubmissionStale, InvalidPrice, InvalidDecimals, etc.

- **storage.rs**: Persistent and instance storage helpers
  - Submitter registry (instance)
  - Feed latest values (instance, hot path)
  - Feed history (persistent, for full audit trail)
  - Nonce map for replay protection (instance)
  - Staleness window configuration (instance)

- **events.rs**: Event emitters
  - submitter_registered, submitter_deactivated
  - price_submitted
  - staleness_window_set

- **lib.rs**: Main contract with 13 public functions

## Core Features

### 1. Price Submission with Replay Protection
```rust
submit_price(submitter, feed_id, price, decimals, timestamp, nonce) -> Result<u64>
```
- **Submitter check**: Only active registered submitters can submit
- **Nonce validation**: Per-submitter-per-feed nonce (must be expected_nonce + 1)
- **Staleness check**: Timestamp must be within staleness window (default 3600s, configurable)
- **Value validation**: Price >= 0, decimals <= 18
- Returns submission ID on success

### 2. Submitter Management (Admin-only)
```rust
register_submitter(caller, submitter) -> Result<()>
deactivate_submitter(caller, submitter) -> Result<()>
```
- Guarded by `shared::auth::require_admin`
- Maintains active/inactive state
- Tracks registration timestamp

### 3. Price Reading
```rust
get_latest_price(feed_id) -> Result<FeedLatest>
get_price_history(feed_id, limit) -> Result<Vec<PriceSubmission>>
```
- Latest prices available instantly (instance storage)
- Full history retained for audit (persistent storage)
- History limited by caller-supplied limit to control gas

### 4. Configuration (Admin-only)
```rust
set_staleness_window(caller, seconds) -> Result<()>
get_staleness_window() -> u64
```
- Default: 3600 seconds (1 hour)
- Adjustable per deployment needs

### 5. Query Helpers
```rust
is_submitter_active(submitter) -> bool
get_admin() -> Address
```

## Storage Strategy

**Hot path optimization:**
- Latest feed values stored in instance storage (fast reads)
- Full history in persistent storage (audit trail, lower priority)
- Nonce map in instance (for replay protection validation)

**Data isolation:**
- Submitter registry: `Map<Address, SubmitterInfo>`
- Feed latest: `Map<Symbol, FeedLatest>`
- Feed history: `Map<Symbol, Vec<PriceSubmission>>` (persistent)
- Nonce map: `Map<(Address, Symbol), u64>` (per-submitter-per-feed)
- Staleness window: scalar (default 3600)

## Acceptance Criteria Checklist

✅ **Authorised submitter can write a value and any caller can read it back**
- `register_submitter` grants permission
- `submit_price` validates submitter active status
- `get_latest_price` and `get_price_history` allow any caller

✅ **Unauthorised submitter is rejected**
- `submit_price` checks `is_submitter_active` and returns `SubmitterNotAuthorized`

✅ **Replayed and stale submissions are rejected**
- Replay: Nonce must be exactly `expected_nonce + 1`, else `DuplicateSubmission`
- Staleness: Timestamp + staleness_window >= current_time, else `SubmissionStale`

✅ **Every state change emits an event**
- `register_submitter` → `submitter_registered`
- `deactivate_submitter` → `submitter_deactivated`
- `submit_price` → `price_submitted`
- `set_staleness_window` → `staleness_window_set`

✅ **Payload format matches Trellis-API expectations**
- `PriceSubmission` struct aligns with off-chain payload-signing.service.ts
- Fields: submitter, feed_id, price, decimals, timestamp, nonce
- Submitter registry and active status validation matches API expectations

## Design Decisions

### Multi-Submitter Strategy
- **Current**: Latest submission overwrites previous (simple, matches API design)
- **Not implemented**: Median/quorum aggregation (deferred to follow-up issue per task description)

### Nonce Scope
- **Chosen**: Per-submitter-per-feed (independent feed sequences)
- **Rationale**: Allows feeds to advance independently; different feeds don't block each other

### Storage Tiers
- **Chosen**: Latest in instance, history in persistent
- **Rationale**: Hot path (price reads) is instant; audit trail kept for compliance

## Public API

All functions follow Soroban conventions:
- Auth checks: `require_auth()` on submitter for submissions
- Admin checks: `shared::auth::require_admin()` for management functions
- Error handling: Result types with detailed OracleError codes
- Ledger integration: Uses `env.ledger().timestamp()` for staleness validation

## Event Schema

All events use dual-topic pattern:
```
(symbol_short!("oracle"), symbol_short!("<action>"))
```

Examples:
- `("oracle", "sub_reg")` → submitter_registered
- `("oracle", "sub_del")` → submitter_deactivated
- `("oracle", "price")` → price_submitted
- `("oracle", "stale")` → staleness_window_set

## Testing

Integration tests in `test_oracle.rs` verify:
1. Authorized submission and read
2. Unauthorized submitter rejection
3. Replay protection
4. Staleness validation
5. Event emission (via behavior verification)
6. Payload structure alignment

Build: `cargo build --release` ✅ (no errors)

## Future Enhancements

Per task description, multi-submitter aggregation (median/quorum) lands in follow-up issue.
