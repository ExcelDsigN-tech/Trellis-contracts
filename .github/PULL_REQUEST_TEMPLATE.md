## Description

Please include a summary of the changes and the related issue. Please also include relevant motivation and context.

Fixes #(issue number)

**Affected crate(s):**

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (changes a contract function, event, or storage layout)
- [ ] Documentation update
- [ ] Performance improvement (e.g. wasm size / resource usage)
- [ ] Code refactoring (no functional changes)

## Checklist

Each item below is enforced by a job in `.github/workflows/ci.yml`.

- [ ] `cargo build --release` succeeds
- [ ] Tests added or updated, and `cargo test --workspace` passes
- [ ] `cargo fmt --all -- --check` is clean
- [ ] `cargo clippy --workspace -- -D warnings` is clean
- [ ] `./scripts/security/run-audit.sh --skip-wasm` passes (any new allowlist entry has a rationale and an expiry)
- [ ] Docs updated (README / crate docs / SECURITY.md) where behaviour changed

## On-chain Interface

- [ ] No emitted event topic or payload changed. If one did, explain why and add a migration note.
- [ ] No storage layout change. If there is one, describe the migration.

## Additional Notes

Add any other information that's important for reviewers to understand this PR.
