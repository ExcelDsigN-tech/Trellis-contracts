# Upgradeability Module

> System-wide upgrade registry and coordinator for the Alian Structure
> Soroban smart-contract suite.

---

## Overview

The Upgradeability module provides a **registry + coordinator** pattern for
managing safe, audited contract upgrades across the protocol. It is adapted
for Soroban's native upgrade model (no EVM-style `delegatecall`).

### Components

| Component | Purpose |
|-----------|---------|
| `UpgradeabilityContract` | Standalone registry that tracks all upgradeable contracts, versions, WASM hashes, migration hooks, and upgrade history. |
| Migration Hooks | Optional helper contracts that run `pre_upgrade` / `post_upgrade` logic before and after a WASM upgrade. |
| Upgrade Test Utilities | Mock contracts, test harnesses, and simulation helpers in the `testing` crate. |

### Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                   UpgradeabilityContract                     │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │   Registry    │  │  Proposals   │  │  Upgrade History  │  │
│  │              │  │              │  │                   │  │
│  │ contract_id  │  │ proposal_id  │  │ contract_id       │  │
│  │ name         │  │ new_version  │  │ old_version       │  │
│  │ version      │  │ wasm_hash    │  │ new_version       │  │
│  │ wasm_hash    │  │ status       │  │ wasm hashes       │  │
│  │ hook_addr    │  │ proposer     │  │ executor          │  │
│  └──────────────┘  └──────────────┘  └───────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │              Authorization Layer                      │    │
│  │  Upgrader role required for propose/execute          │    │
│  │  Admin role required for register/set_hook           │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
     ┌──────────────┐ ┌──────────┐ ┌──────────────┐
     │  aid_contract │ │treasury  │ │ referral     │
     │              │ │_contract │ │ _contract    │
     │  upgrade()   │ │upgrade() │ │  upgrade()   │
     └──────────────┘ └──────────┘ └──────────────┘
```

Each upgradeable contract exposes its own `upgrade` entry point that calls
`env.deployer().update_current_contract_wasm()`. The UpgradeabilityContract
validates that the upgrade has been properly authorized through the registry
before allowing the WASM update.

---

## Upgrade Flow

### 1. Registration

Register a contract for upgrade management:

```rust
// In tests:
client.register_contract(
    &admin,
    &contract_id,
    symbol_short!("aid"),  // logical name
    &1,                     // initial version
    &initial_wasm_hash,
);
```

### 2. Propose Upgrade

Create an upgrade proposal (requires `Upgrader` role):

```rust
let proposal_id = client.propose_upgrade(
    &upgrader,
    &contract_id,
    &new_wasm_hash,
    &2,  // new version (must be > current)
    &soroban_sdk::String::from_str(&env, "security patch"),
);
```

### 3. Execute Upgrade

Execute the proposal (requires `Upgrader` role):

```rust
client.execute_upgrade(&upgrader, &proposal_id);
```

This triggers:
1. Pre-upgrade migration hook (if configured)
2. Registry update with new version/hash
3. Post-upgrade migration hook (if configured)
4. History recording
5. Event emission

### 4. WASM Update

The target contract must call its own upgrade function:

```rust
pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
    // Verify authorization via the registry.
    env.invoke_contract(
        &registry_id,
        &Symbol::new(&env, "verify_upgrade_authorization"),
        (env.current_contract_address(), caller, new_wasm_hash.clone()),
    );

    // Perform the actual WASM update.
    env.deployer().update_current_contract_wasm(new_wasm_hash);
}
```

---

## Migration Hooks

Migration hooks allow pre/post upgrade logic (state transformations, validations).

### Hook Interface

A migration hook contract must implement:

```rust
/// Called before the upgrade. Return `true` to approve, `false` to reject.
pub fn pre_upg(env: Env, old_version: u32, new_version: u32) -> bool;

/// Called after the upgrade. Used for state migrations.
pub fn pst_upg(env: Env, old_version: u32, new_version: u32);
```

### Example: State Migration Hook

```rust
#[contract]
pub struct V1ToV2MigrationHook;

#[contractimpl]
impl V1ToV2MigrationHook {
    pub fn pre_upg(env: Env, old_version: u32, new_version: u32) -> bool {
        // Validate preconditions.
        old_version == 1 && new_version == 2
    }

    pub fn pst_upg(env: Env, old_version: u32, new_version: u32) {
        if old_version == 1 && new_version == 2 {
            // Migrate state from V1 layout to V2 layout.
            // e.g., transform storage keys, update data structures.
        }
    }
}
```

### Registering a Hook

```rust
client.set_migration_hook(&admin, &contract_id, &hook_contract_addr);
```

---

## Upgrade Checklist

### Pre-Upgrade

- [ ] **Code Review** — New WASM has been audited and reviewed.
- [ ] **Test Coverage** — All unit and integration tests pass.
- [ ] **Migration Plan** — Documented state changes between versions.
- [ ] **Rollback Plan** — Documented procedure to revert if needed.
- [ ] **Migration Hook** — Hook contract tested and deployed (if needed).
- [ ] **Storage Layout** — Verified storage layout compatibility.
- [ ] **Gas Estimation** — Estimated gas costs for migration transactions.
- [ ] **Testnet Deployment** — Upgrade tested on testnet first.
- [ ] **Monitoring** — Alerting configured for post-upgrade anomalies.

### During Upgrade

- [ ] **Upload WASM** — `soroban contract upload --wasm new.wasm`.
- [ ] **Propose** — Create upgrade proposal via registry.
- [ ] **Approve** — Ensure sufficient signers have approved (if multi-sig).
- [ ] **Execute** — Execute the upgrade via registry.
- [ ] **Update WASM** — Call the target contract's `upgrade` function.

### Post-Upgrade

- [ ] **Verify Version** — Confirm `get_version` returns expected value.
- [ ] **Verify WASM Hash** — Confirm `get_wasm_hash` matches new hash.
- [ ] **Functional Tests** — Run critical-path functional tests.
- [ ] **Event Verification** — Confirm upgrade events emitted correctly.
- [ ] **History Check** — Verify upgrade recorded in history.
- [ ] **Monitor** — Watch for errors/anomalies for 24 hours.

---

## Security Considerations

### Authorization

- **Upgrader Role Required** — Only addresses holding the `Upgrader` role
  can propose and execute upgrades. This role is granted via the governance
  contract's multi-sig proposal flow.
- **Admin Role for Registry Management** — Registering contracts and setting
  migration hooks requires the `Admin` role.
- **Role Separation** — Upgrade authorization is separate from treasury,
  pausing, and other administrative functions.

### Integrity

- **WASM Hash Verification** — The registry stores and verifies WASM hashes.
  An upgrade cannot proceed if the new hash matches the current one (no-op
  prevention).
- **Version Monotonicity** — New version numbers must be strictly greater
  than the current version.
- **Pending Upgrade Lock** — Only one pending upgrade per contract at a time.
  Prevents race conditions.
- **Pre-Upgrade Validation** — Migration hooks can reject upgrades that fail
  validation checks.

### Auditability

- **Full History** — Every upgrade is recorded with old/new versions, WASM
  hashes, executor, and timestamp.
- **Event Emission** — All registry operations emit events for off-chain
  indexing: `registered`, `proposed`, `executed`, `hook_set`, `rollback`.
- **Proposal Tracking** — Upgrade proposals include proposer address, note,
  and execution status.

### Rollback

- **Cancel Proposals** — Pending proposals can be cancelled by the proposer
  or an admin before execution.
- **Historical WASM Hashes** — Previous WASM hashes are preserved in the
  upgrade history, enabling rollback to a known-good version.
- **Manual Rollback** — To rollback, propose a new upgrade with the old WASM
  hash and bump the version number.

### Migration Hook Safety

- **Hook Failure Aborts Upgrade** — If a migration hook returns `false` or
  panics, the upgrade is aborted.
- **Pre vs Post** — `pre_upgrade` runs before WASM update; `post_upgrade`
  runs after. A failure in either aborts the upgrade.
- **Hook Contract Immutability** — Hook contracts should be deployed once
  and never upgraded themselves.

### Known Limitations

1. **No Atomic WASM + State Upgrade** — In Soroban, the WASM update and
   state migration are separate transactions. A failure between them leaves
   the contract on new WASM with old state. Migration hooks help mitigate
   this but cannot make it atomic.

2. **Cross-Contract Hook Calls** — Migration hooks execute as cross-contract
   calls, which have gas overhead and can fail independently.

3. **No Automatic Rollback** — If an upgrade fails post-execution, manual
   intervention is required to propose a rollback upgrade.

---

## Scripts

| Script | Purpose |
|--------|---------|
| `scripts/deploy.sh` | Deploy all contracts including upgradeability registry. |
| `scripts/initialize.sh` | Initialize all contracts including upgradeability registry. |
| `scripts/register_upgradeable.sh` | Register a contract in the upgrade registry. |
| `scripts/upgrade.sh` | Perform a complete upgrade through the registry. |
| `scripts/verify.sh` | Verify WASM artifacts exist after build. |

### Quick Start

```bash
# 1. Build WASM
cargo build --target wasm32v1-none --release

# 2. Deploy upgradeability registry
soroban contract deploy \
  --wasm target/wasm32v1-none/release/upgradeability.wasm \
  --source admin

# 3. Initialize
soroban contract invoke \
  --id <REGISTRY_ID> \
  -- initialize --admin <ADMIN_ADDRESS>

# 4. Register a contract
./scripts/register_upgradeable.sh <REGISTRY_ID> <CONTRACT_ID> "aid" 1 \
  target/wasm32v1-none/release/aid_contract.wasm

# 5. Perform an upgrade
./scripts/upgrade.sh <REGISTRY_ID> <CONTRACT_ID> \
  target/wasm32v1-none/release/aid_contract.wasm 2 "security patch"
```

---

## Testing

Run upgradeability tests:

```bash
# Run the upgradeability contract's own tests
cargo test -p upgradeability

# Run upgrade test utilities
cargo test -p testing -- upgrade

# Run all tests
cargo test
```

### Test Utilities

The `testing` crate provides:

- `MockMigrationHook` — Configurable migration hook for testing.
- `MockUpgradeableContract` — Simulates upgradeable contract behavior.
- `UpgradeTestHarness` — End-to-end upgrade flow test harness.
- `fake_wasm_hash(seed)` — Generate deterministic fake WASM hashes.

---

## Storage Layout

| Key Pattern | Storage Type | Description |
|-------------|-------------|-------------|
| `(KEY_REG_ENTRY, contract_id)` | Instance | Registry entry for a contract. |
| `(KEY_CONTRACT_BY_NAME, name)` | Instance | Map logical name to contract ID. |
| `(KEY_REG_CNT,)` | Instance | Total registered contract count. |
| `(KEY_UPG_PROP, proposal_id)` | Instance | Upgrade proposal details. |
| `(KEY_PROP_CNT,)` | Instance | Total proposal count. |
| `(KEY_PENDING, contract_id)` | Instance | Pending proposal ID for a contract. |
| `(KEY_HOOK, contract_id)` | Instance | Migration hook address. |
| `(KEY_HISTORY, contract_id, proposal_id)` | Instance | Upgrade history record. |

---

## Error Codes

| Code | Name | Description |
|------|------|-------------|
| 900 | `ContractNotRegistered` | Contract not found in registry. |
| 901 | `ContractAlreadyRegistered` | Duplicate name registration. |
| 902 | `NoChangeDetected` | Same version or WASM hash as current. |
| 903 | `ProposalNotFound` | Upgrade proposal does not exist. |
| 904 | `AlreadyExecuted` | Proposal was already executed. |
| 905 | `MigrationHookFailed` | Migration hook call failed. |
| 906 | `AlreadyPending` | A pending upgrade already exists. |
| 907 | `NotUpgrader` | Caller lacks Upgrader role. |
| 908 | `InvalidWasmHash` | Empty or mismatched WASM hash. |
| 909 | `StorageIncompatible` | Storage layout incompatibility. |
| 910 | `InvalidMigrationHook` | Invalid hook contract address. |
