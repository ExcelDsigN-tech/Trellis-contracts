# Security Audit Module

Automated static analysis, reporting and CI gating for the trellis
contract workspace. This module does not change any contract code; it adds the
tooling required to run repeatable security checks over it.

## What runs

| Check | Tool | Severity mapping |
|---|---|---|
| Known vulnerabilities in dependencies (RustSec advisories) | `cargo audit` | every vulnerability -> high (RustSec has no severity field); unmaintained/yanked warnings -> medium |
| Security lint preset on all contract libs | `cargo clippy` (see preset below) | panic-family lints -> high, others -> medium |
| Wasm size inventory per contract (gas-cost proxy) | `cargo build --target wasm32v1-none --release` | informational |

### Clippy security preset

The preset flags constructs that are common sources of contract
vulnerabilities: explicit panics (`clippy::panic`), unchecked unwraps/expects
(`clippy::unwrap_used`, `clippy::expect_used`), potentially overflowing
arithmetic (`clippy::arithmetic_side_effects`) and direct index slicing
(`clippy::indexing_slicing`). It is applied to library targets only so test
code is not gated.

## Running locally

```bash
# Full run with report (requires cargo-audit, clippy, wasm32v1-none target)
./scripts/security/run-audit.sh --report security/reports/local.md

# Skip the (slow) wasm size pass
./scripts/security/run-audit.sh --skip-wasm

# Install the advisory scanner if missing
cargo install cargo-audit --locked
```

Results are written to stdout and, when `--report` is given, assembled into a
Markdown SAR (Security Audit Report).

## Interpreting the report

- **High severity findings** fail CI by default.
- A finding can be suppressed only through an entry in
  [`security/audit-allowlist.toml`](../security/audit-allowlist.toml). Every
  entry requires a human-written `rationale`; entries without one are treated
  as unfixed. Optional `expires` dates turn stale exceptions into failures,
  forcing periodic re-review.
- Medium/informational findings are reported but never gate.

## CI

The `audit` job in `.github/workflows/ci.yml` runs the same script on every push,
every pull request and weekly (Monday 06:00 UTC), installs `cargo-audit`, and fails when a
high-severity finding has no valid allowlist entry.

## Allowlisting a finding

```toml
[[exceptions]]
id = "RUSTSEC-2024-0421"            # advisory id or clippy lint name
path = "contracts/treasury-contract" # optional scope filter
severity = "high"                    # must match the finding's severity
rationale = "ID is validated upstream before reaching this arithmetic"
expires = "2026-12-31"               # optional, forces re-review after this date
```
