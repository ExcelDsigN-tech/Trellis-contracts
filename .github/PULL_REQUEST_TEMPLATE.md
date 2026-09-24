## Summary

<!-- What does this change and why? -->

Closes #

## Affected crate(s)

## Checklist

<!-- Each item below is enforced by a job in .github/workflows/ci.yml. -->

- [ ] `cargo build --release` succeeds
- [ ] Tests added or updated, and `cargo test --workspace` passes
- [ ] `cargo fmt --all -- --check` is clean
- [ ] `cargo clippy --workspace -- -D warnings` is clean
- [ ] `./scripts/security/run-audit.sh --skip-wasm` passes (any new allowlist entry has a rationale and an expiry)
- [ ] Docs updated (README / crate docs / SECURITY.md) where behaviour changed

## On-chain interface

- [ ] No emitted event topic or payload changed. If one did, explain why and add a migration note.
- [ ] No storage layout change. If there is one, describe the migration.
