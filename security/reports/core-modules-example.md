# Security Audit Report

- Generated: 2026-08-21T21:16:28Z
- Allowlist: `security/audit-allowlist.toml`

## Dependency advisories (cargo audit)

No known advisories.

## Security lint preset (clippy, lib targets)

```
```

## Wasm size inventory (release, gas-cost proxy)

- `aid-contract`: 11552 bytes
- `governance-contract`: 7276 bytes
- `oracle-contract`: 2161 bytes
- `rebalancer-contract`: 4046 bytes
- `referral-contract`: 15284 bytes
- `registry-contract`: 4136 bytes
- `treasury-contract`: 10235 bytes

## Summary

Medium/informational findings (non-gating): 9
- GATE: clippy::expect_used at shared\src\auth.rs:60
- GATE: clippy::expect_used at contracts\aid-contract\src\storage.rs:92
- GATE: clippy::expect_used at contracts\aid-contract\src\lib.rs:87

## Dependency advisories (cargo audit)

No known advisories.

## Security lint preset (clippy, lib targets)

```
```

## Wasm size inventory (release, gas-cost proxy)

- `aid-contract`: 11552 bytes
- `governance-contract`: 7276 bytes
- `oracle-contract`: 2161 bytes
- `rebalancer-contract`: 4046 bytes
- `referral-contract`: 15284 bytes
- `registry-contract`: 4136 bytes
- `treasury-contract`: 10235 bytes

## Summary

Medium/informational findings (non-gating): 9
PASS: no ungated high-severity findings.
