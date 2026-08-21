# Batch Operations — Security Review

## Module Overview

The Batch Operations module (`shared/src/batch.rs`) provides an atomic multi-call
executor for Soroban smart contracts. This document covers the security analysis
of the module's design and implementation.

---

## 1. Reentrancy

### Risk

A malicious or buggy contract could call back into the batch executor while it
is still processing, potentially causing double-execution or inconsistent state.

### Mitigation

The executor uses a **temporary-storage reentrancy guard**:

```rust
const REENTRANCY_KEY: Symbol = symbol_short!("bat_lck");

fn acquire_reentrancy_guard(env: &Env) -> Result<(), BatchError> {
    if crate::storage::temporary_has(env, &REENTRANCY_KEY) {
        return Err(BatchError::ReentrancyDetected);
    }
    crate::storage::temporary_set(env, &REENTRANCY_KEY, &true);
    Ok(())
}

fn release_reentrancy_guard(env: &Env) {
    crate::storage::temporary_remove(env, &REENTRANCY_KEY);
}
```

**Why temporary storage?**
- The guard only needs to live for the duration of the transaction.
- Temporary storage entries expire naturally after the transaction completes,
  so there is no risk of stale locks persisting.
- The lock is always released in the `execute_multi_transfer` /
  `execute_multi_invoke` functions after processing completes (or fails).

**Limitations:**
- The guard prevents reentrant calls to the *same batch executor function*.
  It does not prevent reentrancy between *different* batch operations if they
  use different lock keys (by design — each entry point should use its own key
  or share the same one).

### Recommendation

All contracts that wrap the batch executor should either:
1. Use the same `REENTRANCY_KEY`, or
2. Implement their own reentrancy guard at the contract level.

---

## 2. Delegatecall Risks

### Analysis

The Batch Operations module does **not** use `delegatecall` or any equivalent
mechanism. Soroban does not support delegatecall-style patterns.

All cross-contract calls use Soroban's standard `invoke_contract` /
`try_invoke_contract` interfaces, which:

- Execute in the **target contract's** context (storage, authorization).
- Maintain **full contract isolation** — the batch executor cannot modify
  the target contract's storage directly.
- Are subject to Soroban's standard authorization checks.

### Implications

- A compromised batch executor **cannot** modify arbitrary contract storage.
- Each target contract independently validates and authorizes operations.
- There is no shared execution context that could be exploited.

### Recommendation

No additional mitigation is needed for delegatecall risks. The Soroban
execution model inherently prevents this class of attack.

---

## 3. Gas Protection / Abuse Prevention

### Risk

An attacker could submit an excessively large batch to consume disproportionate
gas resources, potentially DoS-ing the network or other transactions.

### Mitigation

Two layers of protection:

1. **Per-call gas limit**: Soroban enforces a per-transaction gas budget.
   Each cross-contract call (`invoke_contract`) consumes gas, so a large batch
   will hit the transaction gas limit naturally.

2. **Operation count cap**: The `BatchConfig.max_operations` field limits the
   number of operations. The absolute maximum is enforced at
   `ABSOLUTE_MAX_BATCH_SIZE = 100`. Callers can set tighter limits via
   `BatchConfig::max_operations`.

```rust
pub const ABSOLUTE_MAX_BATCH_SIZE: u32 = 100;

fn validate_config(config: &BatchConfig) -> Result<(), BatchError> {
    if config.max_operations == 0 || config.max_operations > ABSOLUTE_MAX_BATCH_SIZE {
        return Err(BatchError::InvalidConfig);
    }
    Ok(())
}
```

### Recommendation

- Set `max_operations` to the minimum required for your use case.
- Monitor batch sizes in production and adjust limits as needed.
- Consider adding per-transaction gas accounting if complex batches
  are expected.

---

## 4. Input Validation

### Mitigation

All inputs are validated before execution:

- **Empty batches**: Rejected with `BatchError::EmptyBatch`.
- **Oversized batches**: Rejected with `BatchError::BatchTooLarge`.
- **Invalid config**: Rejected with `BatchError::InvalidConfig` (zero or
  excessive `max_operations`).
- **Invalid transfers**: Rejected with `BatchError::InvalidOperation`
  (zero or negative amounts).

Validation happens **before** the reentrancy guard is acquired, so invalid
inputs don't hold the lock.

### Note on Non-Atomic Mode

In non-atomic mode, individual operation failures are caught via
`try_invoke_contract` and recorded in `OperationResult::Failure`. The batch
continues executing subsequent operations. This is by design — callers should
review the `BatchResult` to determine if all operations succeeded.

---

## 5. Authorization Model

### Token Transfers

For `execute_multi_transfer`, the `caller` address must have pre-authorized
the batch contract to transfer tokens on their behalf. This is handled by
Soroban's authorization tree:

1. The user signs a transaction that calls the batch contract.
2. The batch contract calls `token.transfer(&caller, &to, &amount)`.
3. Soroban checks that `caller` authorized this specific transfer.
4. If authorization fails, the transfer panics (atomic mode) or returns
   an error (non-atomic mode).

### Generic Invocations

For `execute_multi_invoke`, authorization for each sub-call is handled by
Soroban's auth tree. The batch executor does not manage authorization — it
delegates to the target contracts.

### Anti-Spoofing

The `BatchTransfer` struct does **not** include a `from` field. The `from`
address is always the `caller` parameter passed to `execute_multi_transfer`.
This prevents an attacker from specifying an arbitrary `from` address.

---

## 6. Event Emission

The batch executor emits events after completion:

| Event | When | Data |
|-------|------|------|
| `BATCH_COMPLETED` | All operations succeeded, or batch was reverted | `(total, succeeded, failed, reverted)` |
| `BATCH_PARTIAL` | Non-atomic batch completed with some failures | `(total, succeeded, failed)` |

Events enable off-chain monitoring and auditing of batch operations.

---

## 7. Failure Semantics

### Atomic Mode

- Uses `invoke_contract` which panics on failure.
- A single failure causes the **entire transaction to revert**, including
  all state changes made by the batch executor.
- This is Soroban's native atomicity guarantee — no additional logic needed.

### Non-Atomic Mode

- Uses `try_invoke_contract` to catch individual failures.
- Successful operations persist; failed operations are recorded.
- The batch executor returns a `BatchResult` with `reverted: false`.

### Validation Failures

- Config and input validation errors are returned as `Err(BatchError)`.
- These occur before any cross-contract calls are made.

---

## 8. Known Limitations

1. **No partial revert in atomic mode**: Soroban's execution model does not
   support reverting individual operations within a transaction. Atomic mode
   reverts the entire transaction on any failure.

2. **No gas refund on failure**: If an atomic batch fails partway through,
   the gas consumed by successful operations is not refunded.

3. **Reentrancy guard scope**: The temporary-storage guard only prevents
   reentrancy within the same transaction. Cross-transaction reentrancy
   is not applicable (temporary storage is cleared after each transaction).

4. **No built-in retry logic**: Failed operations in non-atomic mode are
   not retried. Callers must implement retry logic if needed.

---

## 9. Audit Recommendations

- [ ] Verify that all cross-contract calls use the correct authorization model.
- [ ] Confirm that the reentrancy guard is correctly acquired and released
  in all code paths (including early returns and panics).
- [ ] Test with realistic batch sizes to ensure gas limits are sufficient.
- [ ] Monitor event emissions for anomalous batch patterns in production.
- [ ] Review any contracts that wrap the batch executor for additional
  security considerations.
