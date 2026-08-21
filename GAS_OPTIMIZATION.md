# Gas Optimization Guide

> A living checklist for contributors adding new features or modifying
> existing Soroban smart contracts in this workspace.

---

## Table of Contents

- [Principles](#principles)
- [Checklist for New Features](#checklist-for-new-features)
- [Identified Hotspots & Applied Optimizations](#identified-hotspots--applied-optimizations)
- [Benchmark Infrastructure](#benchmark-infrastructure)
- [CI Gas Regression Checks](#ci-gas-regression-checks)

---

## Principles

Soroban "gas" is measured in **compute units (CUs)** and **storage I/O**. Every
host function call (storage read/write, event publish, cross-contract invoke)
has a CU cost. The primary levers for reducing gas are:

1. **Fewer storage I/O operations** — each `env.storage().*()` call is a host
   syscall. Combine reads, avoid redundant reads, and skip writes when the value
   hasn't changed.
2. **Cheapest-fail validation** — reorder guard checks so trivial rejections
   (amount <= 0) happen before expensive ones (auth commitments, storage reads).
3. **Short-circuit early** — return immediately on the most common failure
   paths; don't fall through to expensive operations.
4. **Conditional TTL bumps** — use `persistent_read` (no TTL bump) for read-
   only queries and loop-internal reads. Use `persistent_get` only when you
   need the entry to stay alive.
5. **Pre-cache stable data** — if a value is read multiple times in a loop and
   doesn't change, read it once before the loop.
6. **Avoid unnecessary serialization** — don't deserialize a full collection
   when a lightweight check (e.g., last-element peek) suffices.

---

## Checklist for New Features

Use this checklist before submitting a PR that adds or modifies contract logic.

### Storage

- [ ] **Read-write pairing**: Does every `persistent_set` have a corresponding
  `persistent_get` (or `persistent_read`)? If you write but never read, the
  TTL bump is wasted gas.
- [ ] **`persistent_read` vs `persistent_get`**: Are you using `persistent_read`
  for query-only paths and loop-internal reads? Only use `persistent_get`
  when you need the TTL extension.
- [ ] **Batch reads**: Can you read all needed values before a loop instead of
  re-reading inside it? (See referral `accrue` tier BPS caching.)
- [ ] **Avoid full-collection deserialization**: If you only need to check
  whether an element exists, use a targeted check (e.g., peek at the last
  element) instead of deserializing the entire Vec/Map.

### Validation Ordering

- [ ] **Cheapest first**: Arrange guard checks from cheapest to most expensive.
  Recommended order:
  1. Amount/value bounds (`amount <= 0`, range checks)
  2. State flags (is_paused, status enum match)
  3. Storage reads (lookup balance, config)
  4. Auth commitment (`require_auth()`, `require_role()`, `require_admin()`)
  5. Cross-contract calls (token transfers, treasury interactions)
- [ ] **Short-circuit on common failures**: If 80% of failures are "amount
  zero", check that first.

### Events

- [ ] **Emit once**: Don't emit duplicate events for the same logical action.
  Prefer the typed helpers in `shared::events` over the generic `emit()`.
- [ ] **Event payload size**: Keep event data compact. Avoid embedding large
  structs; use IDs that can be looked up off-chain.

### Math & Types

- [ ] **Use `#[inline]`** on small hot-path helper functions (< 5 statements).
- [ ] **Derive `Copy`** on small enums used in `match` arms to avoid cloning.
- [ ] **Use `checked_*` arithmetic** — overflow panics are cheaper than
  underflow panics but both waste gas on failure. Design so the common path
  never overflows.

### Cross-Contract Calls

- [ ] **Minimize calls**: Each `env.invoke_contract()` or
  `env.try_invoke_contract()` is expensive. Batch operations if possible.
- [ ] **Cache addresses**: Don't re-read a stored contract address if you
  already have it in a local variable.

---

## Identified Hotspots & Applied Optimizations

### 1. Shared Storage: `persistent_read` (NEW)

**File**: `shared/src/storage.rs`

**Problem**: `persistent_get` always calls `extend_ttl`, even for read-only
query paths where the entry was recently written and TTL is fresh.

**Solution**: Added `persistent_read<K, V>()` — a lightweight read that skips
the TTL extension entirely.

**Impact**: Every read-only query across all contracts saves one host syscall.
This affects:
- `accrued_balance()` / `lifetime_accrued()` getters in referral contract
- `get_aid()` query in aid contract
- `get_contract()` / `get_version_history()` in registry contract
- Loop-internal reads in referral `accrue`

### 2. Referral Contract: `accrue` Tier BPS Pre-Caching

**File**: `contracts/referral-contract/src/lib.rs`

**Before**: Each iteration of the tier loop called `read_tier_bps()`, which is
an instance-storage read. With N tiers, that's N redundant reads.

**After**: All tier BPS values are pre-cached into a `Vec` before the loop.
Each iteration reads from the local Vec (essentially free) instead of storage.

**Estimated savings**: ~2 host syscalls per tier × number of tiers.

### 3. Registry Contract: `set_contract` Fast-Path

**File**: `contracts/registry-contract/src/lib.rs`

**Before**: Every `set_contract` call deserialized the full history `Vec` and
ran a linear scan to check for duplicates.

**After**: The last version in the Vec is checked first (O(1)). If it matches,
the full Vec deserialization + linear scan is skipped entirely. This is the
common case when re-registering the same version.

**Estimated savings**: 1 Vec deserialization + 1 linear scan in the common
(fast-path) case.

### 4. Treasury Contract: Validation Reordering

**File**: `contracts/treasury-contract/src/lib.rs`

**Before**: `withdraw`, `deposit`, `distribute_reward`, and
`emergency_withdraw` ran the auth check (`require_role` / `require_admin`)
before cheap validation (amount <= 0).

**After**: Cheap validation runs first. A zero/negative amount is rejected
with a single integer comparison before the expensive auth commitment.

**Impact**: Saves the full auth cost on every trivial reject. Auth involves
crypto verification in the host — the most expensive single operation.

---

## Benchmark Infrastructure

### Running Benchmarks

```bash
# Run all gas benchmarks
cargo test -p aid-contract -- gas_bench
cargo test -p treasury-contract -- gas_bench
cargo test -p referral-contract -- gas_bench
cargo test -p registry-contract -- gas_bench
cargo test -p shared -- persistent_read

# Run all benchmarks at once
cargo test -- gas_bench persistent_read
```

### What the Benchmarks Measure

Since Soroban's test host doesn't expose raw CU counts, the benchmarks measure
**behavioral correctness of optimized paths**:

- `gas_bench_accrue_3_tier`: Verifies pre-cached tier BPS produces correct
  results for a 3-tier chain.
- `gas_bench_accrue_max_depth`: Stress-tests the 10-tier worst case.
- `gas_bench_claim_rewards`: Verifies the claim flow reads/writes correctly.
- `gas_bench_withdraw_rejects_zero_before_auth`: Verifies zero-amount is
  rejected before auth.
- `gas_bench_deposit_rejects_zero_before_auth`: Same pattern for deposit.
- `gas_bench_distribute_reward_rejects_zero_before_auth`: Same pattern.
- `gas_bench_emergency_withdraw_rejects_zero_before_auth`: Same pattern.
- `gas_bench_set_contract_same_version_skips_history`: Verifies the fast-path
  doesn't corrupt history.
- `persistent_read_*`: Verifies `persistent_read` returns correct values.

### Adding New Benchmarks

When adding a new contract function:

1. Write a `#[test]` named `gas_bench_<function_name>()` that exercises the
   optimized path.
2. Test both the happy path and the most common error path.
3. If the function reads storage in a loop, verify the cached values are
   correct.

---

## CI Gas Regression Checks

Gas regression checks run via the CI workflow at `.github/workflows/gas-check.yml`.
The workflow:

1. Runs `cargo test` for all contracts.
2. Runs the gas benchmark tests specifically (`-- gas_bench persistent_read`).
3. Fails the build if any benchmark test fails.

This ensures that future changes don't silently break gas optimizations.

---

## Soroban Cost Reference

Approximate CU costs for common host operations (subject to change per
Soroban version):

| Operation                    | ~CU Cost |
|------------------------------|----------|
| Instance storage read        | ~500     |
| Instance storage write       | ~1,000   |
| Persistent storage read      | ~1,000   |
| Persistent storage write     | ~2,000   |
| Persistent storage extend_ttl| ~500     |
| Event publish                | ~500     |
| `require_auth()`             | ~1,000+  |
| Cross-contract invoke        | ~5,000+  |

These costs are additive — every syscall counts. The optimizations in this
workspace target the highest-frequency operations (storage reads in loops,
auth on trivial rejects, redundant deserialization).
