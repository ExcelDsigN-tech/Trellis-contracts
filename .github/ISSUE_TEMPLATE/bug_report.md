---
name: Bug Report
about: Report a bug to help us improve
labels: bug, needs-triage
title: "[Bug]: "
---

## Bug Report

> **Security vulnerability?** Do not file it here. Follow [SECURITY.md](../../SECURITY.md) to report it privately.

### Describe the Bug
A clear and concise description of what the bug is.

### Affected Crate
e.g. `aid-contract`, `treasury-contract`, `payments-contract`, `shared`

### Steps to Reproduce
Steps to reproduce the behavior:
1. Deploy / invoke '...'
2. Call '....' with arguments '....'
3. See error

A failing `cargo test` case is the most useful reproduction.

### Expected Behavior
A clear and concise description of what you expected to happen.

### Actual Behavior
What actually happened. Include the error code, panic message and emitted events if applicable.

### Environment
 - Network: [testnet / futurenet / local sandbox]
 - Contract ID: [e.g. C...]
 - Transaction hash: [if reproduced on-chain]
 - Commit: [output of `git rev-parse HEAD`]
 - Rust / stellar-cli version: [e.g. 1.89.0 / 23.0.0]

### Additional Context
Add any other context about the problem here. This could include:
- Relevant logs
- What you were trying to achieve
- Any workarounds you've found
