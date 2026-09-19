# Trellis — Soroban smart contracts — Wave Program issues

Twenty issues for the Trellis contracts repository, grounded in the current state
of `main`. Each one cites the specific file, line count or test count that
prompted it, so a contributor can verify the problem before starting.

These are staged for release, not posted all at once. Each stage is a batch you can push when the previous one has been picked up.

| Stage | Issues | When to post |
| --- | --- | --- |
| 1 | 7 (#1–#7) | Ready to post now |
| 2 | 8 (#8–#15) | Core contributor work |
| 3 | 5 (#16–#20) | Needs a maintainer decision first |

Create them with `.github/create-issues.sh <stage>` — see the bottom of this file.

---

## Stage 1 — Ready to post now

Self-contained, low-risk, and reviewable in a single pass. Most need no prior knowledge of the codebase. Post these first so the first wave of contributors has somewhere to land.

### 1. Add the MIT LICENSE file the README already promises

**Labels:** `good first issue`, `documentation`, `legal`

The README carries an MIT licence badge and its License section says
"Licensed under the MIT License. See LICENSE for details." — but there is no
`LICENSE` file in the repository. `ls` at the root confirms it: the file has
never existed.

## Why this matters

Without a licence file the code is, legally, all-rights-reserved. Nobody can
safely fork, audit or build on it, GitHub shows no licence in the sidebar, and
most grant and accelerator programmes treat a missing licence as disqualifying.
For a project whose whole pitch is verifiable public-good infrastructure, this
is the single cheapest credibility fix available.

## Tasks

- [ ] Add a `LICENSE` file at the repository root with the standard MIT text
- [ ] Set the copyright line to `Copyright (c) 2026 Trellis`
- [ ] Confirm GitHub's sidebar shows "MIT License" after merge
- [ ] Check the sibling repositories use the same licence, and open an issue there if they disagree

## Files to look at

- `LICENSE (new)`
- `README.md`

## Acceptance criteria

- A `LICENSE` file exists at the root containing the unmodified MIT licence text
- GitHub's repository sidebar displays the licence
- The README's License section and badge still match the file

## Notes

Note that `Trellis-API` is Apache-2.0 while the README here claims MIT. Flag it
in the thread if the licences should be aligned — that is a maintainer decision,
not part of this issue.

---

**Skills:** Markdown  
**Estimated effort:** 15 minutes

### 2. CI never runs the test suite — add a `cargo test` job

**Labels:** `bug`, `ci`, `good first issue`

`.github/workflows/ci.yml` defines exactly three jobs: `build`
(`cargo build --release`), `clippy` (`cargo clippy --workspace -- -D warnings`)
and `format` (`cargo fmt --all -- --check`).

There is no `cargo test` anywhere in the workflow. The repository has **324
tests** across the contract crates and `shared/` — 41 in `upgradeability`, 35 in
`governance-contract`, 32 in `access-control`, 100 in `shared` — and not one of
them runs on a pull request.

## Why this matters

Every test in this repository is currently decorative. A PR can delete an
assertion, break `claim_aid`, or regress the treasury accounting and CI will
still go green as long as it compiles and is formatted. On contracts that move
donor money, that is the most serious process gap in the repo.

## Tasks

- [ ] Add a `test` job to `.github/workflows/ci.yml` running `cargo test --workspace`
- [ ] Reuse the existing `dtolnay/rust-toolchain@stable` and `actions/cache@v4` steps for consistency
- [ ] Confirm the job fails when a test is deliberately broken, then revert that change
- [ ] Make the job required for merge in the branch protection settings (maintainer step — note it in the PR)

## Files to look at

- `.github/workflows/ci.yml`

## Acceptance criteria

- `cargo test --workspace` runs on every push and pull request to `main` and `develop`
- A deliberately failing test causes a red check (demonstrate in the PR description)
- The job uses the same cache key strategy as the existing jobs so it does not slow CI down

---

**Skills:** GitHub Actions, Rust  
**Estimated effort:** 1 hour

### 3. Add CONTRIBUTING.md

**Labels:** `good first issue`, `documentation`

`Trellis-frontend` and `Trellis-API` both have a `CONTRIBUTING.md` (171 and 124
lines respectively). This repository has none — no setup instructions beyond the
README, no branch naming convention, no PR expectations, no explanation of how
to run the test suite or what reviewers look for.

## Why this matters

This is the repository most likely to intimidate a newcomer — Rust, Soroban, and
money-handling logic. A contributor who cannot work out how to build and test
locally will not open a PR at all. It is also the first thing Wave reviewers
check when judging whether a project is ready to receive contributors.

## Tasks

- [ ] Document local setup: Rust toolchain (see `rust-toolchain.toml`), the `wasm32v1-none` target, and the Soroban CLI
- [ ] Document how to build (`cargo build --release`) and test (`cargo test --workspace`)
- [ ] Document how to run a single crate's tests, e.g. `cargo test -p aid-contract`
- [ ] Describe the branch naming and commit message conventions already in use in the git history
- [ ] Explain the PR process: what CI must pass, that new contract logic needs tests, and who reviews
- [ ] Point at `testing/README.md`, which already documents the mocks and fuzzing harnesses
- [ ] Link the new file from the README

## Files to look at

- `CONTRIBUTING.md (new)`
- `README.md`
- `testing/README.md`
- `rust-toolchain.toml`

## Acceptance criteria

- A new contributor can go from `git clone` to a passing `cargo test --workspace` using only CONTRIBUTING.md
- The file is linked from the README
- Conventions described match what the git history actually shows

---

**Skills:** Markdown  
**Estimated effort:** 2 hours

### 4. Add a CODE_OF_CONDUCT.md

**Labels:** `good first issue`, `documentation`, `community`

The other two Trellis repositories each ship a Code of Conduct (76 and 78
lines, both Contributor Covenant). This one does not.

## Why this matters

An open-source project taking outside contributors needs a stated standard of
behaviour and a reporting route, both so participants know what to expect and so
maintainers have something to point at. GitHub also surfaces its presence in the
community profile, which several funding programmes check.

## Tasks

- [ ] Adopt Contributor Covenant v2.1, matching the version used in the sibling repositories
- [ ] Fill in a real reporting contact — do not leave the template placeholder
- [ ] Link it from the README and from CONTRIBUTING.md

## Files to look at

- `CODE_OF_CONDUCT.md (new)`
- `README.md`
- `CONTRIBUTING.md`

## Acceptance criteria

- The file is present and contains a working reporting contact address
- GitHub's community profile shows the Code of Conduct as satisfied
- The version matches `Trellis-API` and `Trellis-frontend` so the three repos are consistent

## Notes

Depends on the CONTRIBUTING.md issue only for the cross-link; it can be merged
independently and linked later.

---

**Skills:** Markdown  
**Estimated effort:** 30 minutes

### 5. Add SECURITY.md with a vulnerability disclosure policy

**Labels:** `good first issue`, `documentation`, `security`

There is no `SECURITY.md` in the repository, so GitHub shows no "Report a
vulnerability" route. The repo does have a `security/` directory with an audit
allowlist and a runner script, but nothing that tells an outside researcher what
to do when they find a bug.

## Why this matters

These contracts hold and move humanitarian aid funds on a public ledger. A
researcher who finds a flaw in `treasury-contract` or `payments-contract` today
has no private channel and will either post it publicly or say nothing. Both
outcomes are bad. A disclosure policy is the minimum responsible posture for a
repository of this kind.

## Tasks

- [ ] Write `SECURITY.md` covering: which versions are supported, how to report privately, and the response time you commit to
- [ ] Enable GitHub private vulnerability reporting in repository settings and reference it
- [ ] State explicitly that the contracts are unaudited and testnet-only, if that is still true
- [ ] Cross-reference `security/README.md` and `SECURITY_BATCH.md`

## Files to look at

- `SECURITY.md (new)`
- `security/README.md`
- `SECURITY_BATCH.md`

## Acceptance criteria

- GitHub's Security tab shows the policy
- The policy names a private reporting channel and a response-time commitment
- The current audit status of the contracts is stated plainly

---

**Skills:** Markdown  
**Estimated effort:** 1 hour

### 6. Add issue and pull request templates

**Labels:** `good first issue`, `documentation`, `community`

`.github/` contains only `workflows/ci.yml`. There are no issue templates and no
pull request template, unlike the other two repositories which both have
`bug_report.md`, `feature_request.md` and a PR template.

## Why this matters

Templates are what make incoming reports actionable. Without them, bug reports on
a contracts repo arrive without the crate name, the network, the contract ID or
the transaction hash — all of which a maintainer then has to ask for. They also
set the tone that this repository expects and welcomes outside contributions.

## Tasks

- [ ] Add `.github/ISSUE_TEMPLATE/bug_report.md` with fields for affected crate, network (testnet/futurenet), contract ID, transaction hash, and reproduction steps
- [ ] Add `.github/ISSUE_TEMPLATE/feature_request.md`
- [ ] Add `.github/ISSUE_TEMPLATE/config.yml` pointing security reports at SECURITY.md rather than the public tracker
- [ ] Add `.github/PULL_REQUEST_TEMPLATE.md` with a checklist: tests added, `cargo fmt` clean, clippy clean, docs updated
- [ ] Use the sibling repositories' templates as a starting point so the three feel like one project

## Files to look at

- `.github/ISSUE_TEMPLATE/ (new)`
- `.github/PULL_REQUEST_TEMPLATE.md (new)`

## Acceptance criteria

- Opening a new issue offers a choice of templates
- The security link in `config.yml` routes reporters away from the public tracker
- The PR template checklist matches what CI actually enforces

---

**Skills:** Markdown, YAML  
**Estimated effort:** 1 hour

### 7. README links to none of its own documentation — add an index

**Labels:** `good first issue`, `documentation`

The repository contains roughly 1,700 lines of documentation beyond the README:
`UPGRADEABILITY.md` (364 lines), `SECURITY_BATCH.md` (241), `GAS_OPTIMIZATION.md`
(233), `testing/README.md` (162), `shared/README.md` (107) and
`security/README.md` (65).

The README links to **none** of them. For comparison, the frontend README has 10
internal documentation links and the API README has 7.

## Why this matters

This documentation is genuinely good and effectively invisible. A contributor
looking for the upgrade model, the gas strategy or the testing harness has to
guess that those files exist and browse the file tree to find them. That is a
pure discoverability loss for work already done.

## Tasks

- [ ] Add a `## Documentation` section to the README with a one-line description of each document and a link
- [ ] Add per-crate links where a crate has its own README (`shared/`, `testing/`, `security/`)
- [ ] Add the index to the README's table of contents
- [ ] Check every link resolves on GitHub after merge (relative paths, correct case)

## Files to look at

- `README.md`
- `UPGRADEABILITY.md`
- `GAS_OPTIMIZATION.md`
- `SECURITY_BATCH.md`
- `testing/README.md`
- `shared/README.md`
- `security/README.md`

## Acceptance criteria

- Every markdown file in the repository is reachable from the README in one click
- Each link has a one-line description of what the reader will find
- No broken relative links (verify on the rendered GitHub page, not locally)

---

**Skills:** Markdown  
**Estimated effort:** 45 minutes

---

## Stage 2 — Core contributor work

The substance of the programme: real features, real test coverage, real bug fixes. Each has a defined acceptance test. Post once Stage 1 has cleared and reviewers have bandwidth.

### 8. The README's "Contract Interfaces" section is empty — write the reference

**Labels:** `documentation`, `help wanted`

The README's table of contents lists "Contract Interfaces" as a section. The
section heading exists. It contains **zero lines of content**.

By contrast, "Events" has 16 lines and "Storage Layout" has 23, so the
surrounding reference material was started and this section was skipped. The
repository exposes a large public surface: `aid-contract` alone has 27 public
functions (`create_aid`, `claim_aid`, `refund_aid`, `update_config`,
`list_aids_by_donor`, and so on), `nft-marketplace` has 34, and there are 11
crates in total.

## Why this matters

This is the single most important piece of documentation a smart contract
repository can have. Anyone integrating — the API team, a wallet, an auditor, a
partner NGO — needs to know each entry point's parameters, authorisation
requirements, failure modes and emitted events. Right now that information
exists only in the Rust source.

## Tasks

- [ ] Agree a per-function format in the issue thread first: signature, description, auth required, parameters, returns, errors, events emitted
- [ ] Document `aid-contract` first — it is the core module and the best template for the rest
- [ ] Work through the remaining crates: treasury, referral, governance, registry, payments, access-control, upgradeability, nft-marketplace, oracle, rebalancer
- [ ] Cross-reference the existing Events and Storage Layout sections rather than duplicating them
- [ ] Consider generating the skeleton from rustdoc rather than writing it by hand

## Files to look at

- `README.md`
- `contracts/*/src/lib.rs`
- `contracts/aid-contract/src/api.rs`

## Acceptance criteria

- Every public entry point on every contract is documented with parameters, return type, errors and events
- The documented signatures match the code (check against `cargo doc`)
- A reader can integrate against `aid-contract` without opening the Rust source

## Notes

This is large enough to split into eleven separate issues, one per crate, if you
want it to absorb more contributors. Start with `aid-contract` as the reference
implementation and let others follow the pattern.

---

**Skills:** Rust, Soroban, technical writing  
**Estimated effort:** 1–2 days (or split per crate)

### 9. oracle-contract is a 23-line stub — implement price feed submission

**Labels:** `enhancement`, `help wanted`, `core`

`contracts/oracle-contract/src/lib.rs` is 23 lines long and contains exactly one
function:

```rust
pub fn initialize(env: Env, admin: Address) {
    shared::auth::set_admin(&env, &admin);
    emit_module_initialized(&env, symbol_short!("oracle"), 1, &admin, env.ledger().timestamp());
}
```

That is the entire contract. It has no submission, no storage, no aggregation, no
reads. Meanwhile the architecture diagram in the README lists "Oracle Contract"
as one of six core modules, and `Trellis-API` has a whole
`src/blockchain/oracle/` subsystem with payload signing built to submit to it.

## Why this matters

The off-chain side is built and has nothing to talk to. Aid distribution depends
on verified external data — identity verification references, exchange rates,
delivery confirmations — and the oracle is where that data is supposed to become
verifiable on-chain. Until it exists, that link in the trust chain is missing.

## Tasks

- [ ] Agree the data model in the issue thread before coding: what a submission contains, who may submit, how disputes are handled
- [ ] Define storage: submitter registry, per-feed latest value, submission history with timestamps
- [ ] Implement `add_submitter` / `remove_submitter` guarded by the existing `shared::auth` admin checks
- [ ] Implement `submit` with replay protection and a staleness window
- [ ] Implement `get_latest` and `get_at` read paths
- [ ] Emit events for every state change, following the pattern in `shared/src/events.rs`
- [ ] Decide whether multi-submitter aggregation (median/quorum) lands here or in a follow-up
- [ ] Align the payload shape with `Trellis-API`'s `payload-signing.service.ts` so the two actually interoperate

## Files to look at

- `contracts/oracle-contract/src/lib.rs`
- `shared/src/auth.rs`
- `shared/src/events.rs`
- `shared/src/storage.rs`
- `contracts/aid-contract/src/ (as a structural reference)`

## Acceptance criteria

- An authorised submitter can write a value and any caller can read it back
- An unauthorised submitter is rejected
- Replayed and stale submissions are rejected
- Every state change emits an event
- The on-chain payload format matches what the API's signing service produces

## Notes

Pair this with the oracle test-suite issue — they should land together. Look at
`aid-contract` for the house structure: separate `api.rs`, `errors.rs`,
`events.rs`, `storage.rs` and `types.rs` modules rather than one large `lib.rs`.

---

**Skills:** Rust, Soroban, oracle design  
**Estimated effort:** 1–2 weeks

### 10. oracle-contract has zero tests

**Labels:** `test`, `help wanted`

Test counts across the workspace:

| crate | tests |
| --- | --- |
| `shared` | 100 |
| `upgradeability` | 41 |
| `governance-contract` | 35 |
| `access-control` | 32 |
| `nft-marketplace` | 31 |
| `payments-contract` | 19 |
| `aid-contract` | 16 |
| `referral-contract` | 14 |
| `treasury-contract` | 14 |
| `registry-contract` | 11 |
| `rebalancer-contract` | 1 |
| **`oracle-contract`** | **0** |

`oracle-contract` is the only crate in the repository with no test file at all.

## Why this matters

Oracle contracts are a favourite attack surface: stale data, unauthorised
submitters, replayed payloads and manipulated aggregation are all standard
exploits. Shipping one without tests on a contract that gates aid disbursement
would be negligent.

## Tasks

- [ ] Add `contracts/oracle-contract/src/tests.rs` following the structure used in `aid-contract`
- [ ] Cover: initialisation, double initialisation rejected, admin-only mutations
- [ ] Cover: authorised submit succeeds, unauthorised submit panics
- [ ] Cover: reading a value that was never written
- [ ] Cover: stale data past the freshness window
- [ ] Cover: replayed submission rejected
- [ ] Use the mocks and helpers already provided by the `testing/` crate

## Files to look at

- `contracts/oracle-contract/src/tests.rs (new)`
- `testing/README.md`
- `contracts/aid-contract/src/tests.rs (as a reference)`

## Acceptance criteria

- Every public function has at least one success and one failure test
- `cargo test -p oracle-contract` passes
- Coverage for the crate is comparable to `aid-contract`'s

## Notes

Blocked on the oracle implementation issue — the interface must exist first.

---

**Skills:** Rust, Soroban test harness  
**Estimated effort:** 3–5 days

### 11. rebalancer: replace the placeholder slippage predictor

**Labels:** `enhancement`, `help wanted`

`contracts/rebalancer-contract/src/slippage_predictor.rs` is ten lines and
returns a constant:

```rust
pub fn predict_slippage(_asset_pair: (Symbol, Symbol), _amount: u128, env: &Env) -> U256 {
    // Placeholder slippage prediction
    U256::from_u32(env, 1)
}
```

Both real inputs are discarded — note the leading underscores on `_asset_pair`
and `_amount`. Slippage is reported as 1 regardless of the pair being traded or
the size of the trade.

## Why this matters

A rebalancer that assumes constant slippage will systematically misprice large
trades and thin pairs. Any caller relying on this to decide whether a rebalance
is worth executing is being given a number with no relationship to reality — and
it looks like a real calculation, which is worse than an obvious stub.

## Tasks

- [ ] Agree the model in the issue thread: constant-product AMM, depth-based, or oracle-fed
- [ ] Define where liquidity depth comes from — the oracle contract is the obvious source
- [ ] Implement the calculation using the actual asset pair and trade amount
- [ ] Use `shared/src/math.rs` for fixed-point arithmetic rather than rolling new helpers
- [ ] Handle the edge cases: zero amount, unknown pair, insufficient liquidity
- [ ] Add unit tests covering small, large and pathological trades

## Files to look at

- `contracts/rebalancer-contract/src/slippage_predictor.rs`
- `contracts/rebalancer-contract/src/lib.rs`
- `shared/src/math.rs`

## Acceptance criteria

- Predicted slippage varies with both trade size and asset pair
- Large trades against thin liquidity predict materially higher slippage than small ones
- Overflow and division-by-zero are impossible for any input
- The model used is documented in the module's rustdoc

---

**Skills:** Rust, Soroban, DeFi mechanics  
**Estimated effort:** 1 week

### 12. rebalancer: `execute_strategy` ignores the strategy and always returns true

**Labels:** `bug`, `enhancement`, `help wanted`

`contracts/rebalancer-contract/src/strategy_executor.rs`:

```rust
pub fn execute_strategy(env: &Env, _strategy: &ExecutionStrategy, trades: &Vec<Trade>) -> bool {
    for trade in trades.iter() {
        // Placeholder for trade execution
        log_trade(env, &trade, 0, 0);
    }
    true
}
```

The `_strategy` parameter is discarded, so every `ExecutionStrategy` variant
behaves identically. No trade is executed — the loop only emits a log. The
function returns `true` unconditionally, so callers cannot detect failure.

`log_trade` in `logging.rs` has the same problem: it takes `_trade`,
`_actual_price` and `_fee` and discards all three, emitting a generic event with
no trade detail. The hardcoded `0, 0` at the call site is the price and fee.

## Why this matters

This is worse than unimplemented — it reports success. A caller sees `true`,
sees events on-chain, and reasonably concludes the rebalance happened. Nothing
moved. If anything integrates against this before it is finished, the failure
will be silent and the on-chain record will be misleading.

## Tasks

- [ ] Implement the distinct `ExecutionStrategy` variants defined in `lib.rs`
- [ ] Perform actual transfers via `shared/src/payments.rs` rather than only logging
- [ ] Return a result type that can express partial and total failure instead of a bare `bool`
- [ ] Fix `log_trade` to record the real trade, execution price and fee
- [ ] Propagate the real price and fee at the call site instead of `0, 0`
- [ ] Add tests per strategy variant, including a failing-trade path

## Files to look at

- `contracts/rebalancer-contract/src/strategy_executor.rs`
- `contracts/rebalancer-contract/src/logging.rs`
- `contracts/rebalancer-contract/src/lib.rs`
- `shared/src/payments.rs`

## Acceptance criteria

- Each strategy variant produces observably different behaviour
- A failed trade is reported to the caller rather than swallowed
- Emitted events carry the trade, price and fee
- Tests cover every variant plus the failure path

---

**Skills:** Rust, Soroban  
**Estimated effort:** 1 week

### 13. rebalancer-contract has one test for 153 lines and five modules

**Labels:** `test`, `help wanted`

`rebalancer-contract` has a single `#[test]` across five source modules —
`lib.rs`, `fee_calculator.rs`, `slippage_predictor.rs`, `strategy_executor.rs`
and `logging.rs`. Every other contract crate in the workspace has between 11 and
41 tests.

## Why this matters

The rebalancer handles multi-asset trades and fee calculation. `fee_calculator.rs`
in particular does arithmetic on `u128` values derived from user input, which is
exactly the kind of code that needs overflow and rounding tests. One test does
not cover five modules.

## Tasks

- [ ] Add table-driven tests for `calculate_total_fees` covering zero trades, one trade, many trades, and overflow boundaries
- [ ] Add tests for `predict_slippage` once it has a real implementation
- [ ] Add tests for `execute_strategy` covering each variant
- [ ] Add a `rebalance` end-to-end test at the contract level
- [ ] Use the `testing/` crate's helpers rather than hand-rolling fixtures

## Files to look at

- `contracts/rebalancer-contract/src/tests.rs`
- `contracts/rebalancer-contract/src/fee_calculator.rs`
- `testing/README.md`

## Acceptance criteria

- Every module in the crate has test coverage
- Fee arithmetic is tested at overflow boundaries
- `cargo test -p rebalancer-contract` passes

## Notes

The slippage and strategy tests depend on those two implementation issues landing first.

---

**Skills:** Rust, Soroban test harness  
**Estimated effort:** 3–5 days

### 14. Add code coverage reporting to CI

**Labels:** `ci`, `test`, `help wanted`

There is no coverage measurement anywhere in the repository. `cargo tarpaulin`,
`cargo llvm-cov` and any coverage upload step are all absent from
`.github/workflows/ci.yml`.

Test counts per crate vary from 0 to 41, but counts are a poor proxy — nobody
currently knows which code paths in `aid-contract` or `treasury-contract` are
actually exercised.

## Why this matters

Coverage is how you find the untested branch in the refund path before an
auditor does. It also gives the programme a visible, objective signal of
improvement over the course of a wave, which is much more compelling than
"we added some tests".

## Tasks

- [ ] Add a coverage job using `cargo llvm-cov` (better `no_std` support than tarpaulin for Soroban crates)
- [ ] Upload the report to Codecov or as a workflow artifact
- [ ] Add a coverage badge to the README
- [ ] Record the current baseline per crate in the issue thread
- [ ] Set a floor that fails CI on regression — agree the number once the baseline is known

## Files to look at

- `.github/workflows/ci.yml`
- `README.md`

## Acceptance criteria

- Coverage is computed and reported on every pull request
- The current baseline is documented
- CI fails if coverage drops below the agreed floor

## Notes

Depends on the `cargo test` CI issue — there is nothing to measure until tests run.

---

**Skills:** GitHub Actions, Rust tooling  
**Estimated effort:** 1 day

### 15. Add dependency vulnerability scanning to CI

**Labels:** `ci`, `security`, `help wanted`

`scripts/security/run-audit.sh` exists and `security/audit-allowlist.toml` is
checked in, so the intent was clearly there — but nothing in
`.github/workflows/ci.yml` invokes either. `cargo audit` never runs
automatically.

## Why this matters

Soroban contracts pull in a dependency tree that changes underneath you. A
known-vulnerable transitive crate that lands in a lockfile update will go
unnoticed indefinitely, and for on-chain code the cost of shipping one is
paid by users, not by a redeploy.

## Tasks

- [ ] Add an `audit` job to CI invoking the existing `scripts/security/run-audit.sh`
- [ ] Wire in `security/audit-allowlist.toml` so accepted advisories do not re-fail the build
- [ ] Decide whether the job blocks merges or only warns — document the choice in the workflow
- [ ] Add a scheduled weekly run so advisories published after merge still surface
- [ ] Document the triage process in SECURITY.md

## Files to look at

- `.github/workflows/ci.yml`
- `scripts/security/run-audit.sh`
- `security/audit-allowlist.toml`
- `security/README.md`

## Acceptance criteria

- `cargo audit` runs on every PR and on a weekly schedule
- The allowlist suppresses accepted advisories without suppressing new ones
- A newly published advisory produces a visible failure

---

**Skills:** GitHub Actions, Rust tooling  
**Estimated effort:** half a day

---

## Stage 3 — Needs a maintainer decision first

Worth doing, but the approach should be agreed in the issue thread before anyone writes code. Post with a maintainer already assigned to discuss.

### 16. `shared` has both `event.rs` and `events.rs` — reconcile them

**Labels:** `refactor`, `needs discussion`

`shared/src/` contains both `event.rs` and `events.rs`. Two modules one letter
apart is almost always either duplication or an abandoned migration, and in
either case it is a trap: a contributor adding an event has no way to know which
one is correct.

The same directory also has `auth.rs` alongside `test_auth.rs`, and `storage.rs`
alongside `test_storage.rs`, which suggests tests live beside implementations
here — worth confirming that is deliberate while you are in there.

## Why this matters

`shared` is 4,656 lines used by all eleven contract crates. Ambiguity at this
layer propagates everywhere, and event schemas in particular are a public
interface: indexers and the API depend on their exact shape.

## Tasks

- [ ] Establish which module is canonical and what the other was for — check the git history
- [ ] Determine which crates import which
- [ ] Agree the target structure in the issue thread before moving code
- [ ] Consolidate, keeping the emitted event topics and payloads byte-identical
- [ ] Update all importing crates
- [ ] Confirm no emitted event's topic or payload changed — this is an on-chain interface

## Files to look at

- `shared/src/event.rs`
- `shared/src/events.rs`
- `shared/src/lib.rs`
- `contracts/*/src/`

## Acceptance criteria

- One canonical events module
- Every crate compiles and all tests pass
- No change to any emitted event's topic or payload shape (demonstrate in the PR)

## Notes

Changing an event's shape is a breaking change for anything indexing the chain.
If consolidation forces a change, that needs its own discussion and a migration
note — do not fold it in silently.

---

**Skills:** Rust  
**Estimated effort:** 2–3 days

### 17. Add a WASM size budget check to CI

**Labels:** `ci`, `performance`, `needs discussion`

CI builds with `cargo build --release` but never measures the resulting WASM, and
never runs `soroban contract build` or `wasm-opt`. `GAS_OPTIMIZATION.md` (233
lines) discusses optimisation strategy, but nothing enforces it.

## Why this matters

Soroban enforces contract size limits, and deployment cost scales with size. A
dependency bump or an innocuous refactor can push a contract over the limit and
the first you learn of it is a failed deployment. A budget check turns that into
a failed PR instead.

## Tasks

- [ ] Add a CI step running `soroban contract build` for each contract crate
- [ ] Record the optimised size of each `.wasm` artifact
- [ ] Agree per-contract budgets in the issue thread — `nft-marketplace` at 2,532 lines will need more headroom than `oracle-contract`
- [ ] Fail the build when a contract exceeds its budget
- [ ] Comment the size delta on pull requests so reviewers see the cost of a change
- [ ] Cross-reference the approach in GAS_OPTIMIZATION.md

## Files to look at

- `.github/workflows/ci.yml`
- `GAS_OPTIMIZATION.md`
- `contracts/*/Cargo.toml`

## Acceptance criteria

- Every contract's optimised WASM size is reported on each PR
- Exceeding the agreed budget fails CI
- The size delta versus the base branch is visible to reviewers

---

**Skills:** GitHub Actions, Soroban, WASM tooling  
**Estimated effort:** 1–2 days

### 18. Publish rustdoc to GitHub Pages

**Labels:** `documentation`, `ci`, `needs discussion`

Nothing in the repository publishes API documentation. There is no `cargo doc`
step in CI, no Pages deployment, and no docs site for any of the three Trellis
repositories — all documentation lives as markdown inside the repos.

## Why this matters

A browsable, always-current API reference is the cheapest documentation a Rust
project can have, because rustdoc generates it from code that already exists. It
also gives the project a documentation URL to cite in applications and on the
website, which it currently lacks entirely.

## Tasks

- [ ] Add a workflow building `cargo doc --workspace --no-deps` and deploying to Pages
- [ ] Decide whether to publish from `main` only or per release — discuss in the thread
- [ ] Add a landing page that orients a reader across the eleven crates rather than dumping them alphabetically
- [ ] Improve crate-level `//!` docs where they are thin, so the generated output is worth reading
- [ ] Link the published site from the README and from the other two repositories

## Files to look at

- `.github/workflows/ (new)`
- `contracts/*/src/lib.rs`
- `README.md`

## Acceptance criteria

- Documentation is published at a stable URL and rebuilt on every merge to `main`
- The landing page explains how the crates relate
- The URL is linked from all three Trellis repositories

## Notes

Consider whether this should be one docs site across all three repos rather than
three separate ones. That is a bigger decision and belongs in the thread.

---

**Skills:** GitHub Actions, rustdoc  
**Estimated effort:** 1–2 days

### 19. Add a cross-contract integration test for the full aid lifecycle

**Labels:** `test`, `needs discussion`

`tests/integration.rs` exists at the workspace root, and the `testing/` crate
provides mocks, helpers, simulation tools and fuzzing harnesses across 2,002
lines. But the per-crate tests all exercise contracts in isolation.

The README's own description of the system is inherently cross-contract: a donor
creates aid, a recipient claims it, the treasury settles, the referral contract
pays a commission, and access-control gates each step.

## Why this matters

Every bug that matters here lives between contracts, not inside them — a claim
that succeeds while the treasury transfer fails, a referral commission paid
twice, an expired aid that can still be claimed. Isolated unit tests cannot find
those, and they are exactly what an auditor will probe first.

## Tasks

- [ ] Agree the scenarios to cover in the issue thread before writing code
- [ ] Build a fixture deploying all participating contracts and wiring them together
- [ ] Cover the happy path: create → claim → settle → referral payout → treasury reconciliation
- [ ] Cover expiry: aid expires unclaimed → refund → treasury balance restored
- [ ] Cover authorisation: each step attempted by the wrong party
- [ ] Cover partial failure: what happens when the treasury transfer fails mid-flow
- [ ] Assert ledger balances at each step, not just that calls returned

## Files to look at

- `tests/integration.rs`
- `testing/src/`
- `contracts/aid-contract/`
- `contracts/treasury-contract/`
- `contracts/referral-contract/`

## Acceptance criteria

- The full donor-to-recipient flow is exercised end to end in a single test
- Balances are asserted at every step
- Failure paths leave no funds stranded
- The tests run in CI within a reasonable time budget

---

**Skills:** Rust, Soroban, multi-contract testing  
**Estimated effort:** 1–2 weeks

### 20. Add a CHANGELOG and document deployed testnet addresses

**Labels:** `documentation`, `needs discussion`

Two related gaps:

1. There is no `CHANGELOG.md`. `Trellis-frontend` has one; this repository does
   not, despite 155 commits and an upgradeable contract system where knowing
   what changed between versions is a safety matter.
2. No deployed contract addresses are recorded anywhere. Every contract ID in
   the repository is a placeholder of the form `CDXXXXXXXX…`, and the deployment
   scripts target testnet without recording what they produced.

## Why this matters

For an upgradeable contract suite, "which version is deployed at which address"
is operational safety information, not documentation polish. It is also the
first thing a reviewer, integrator or auditor looks for — and its absence reads
as "never actually deployed", whether or not that is true.

## Tasks

- [ ] Add `CHANGELOG.md` in Keep a Changelog format, seeded from the existing git history
- [ ] Decide the versioning scheme — discuss in the thread, given contracts are upgradeable
- [ ] Add a `## Deployments` section to the README: contract, network, address, version, deploy date
- [ ] Record whatever is currently deployed on testnet; if nothing is, say so explicitly
- [ ] Make recording the address a step in `scripts/deploy.sh` so it stays current
- [ ] Cross-reference `UPGRADEABILITY.md`, which documents the upgrade registry

## Files to look at

- `CHANGELOG.md (new)`
- `README.md`
- `scripts/deploy.sh`
- `UPGRADEABILITY.md`

## Acceptance criteria

- A changelog exists and covers at least the most recent release
- Every deployed contract's network, address and version is recorded
- Deploying updates the record rather than requiring a manual edit

---

**Skills:** Markdown, Soroban deployment  
**Estimated effort:** 1–2 days

---

## Posting these

```bash
# from the repository root, with the GitHub CLI authenticated
./.github/create-issues.sh 1     # post stage 1
./.github/create-issues.sh 2     # later
./.github/create-issues.sh 3

./.github/create-issues.sh 1 --dry-run   # print without creating
```

The script reads `.github/wave-issues.json`, which is generated from the same source as this file. Edit the JSON if you want to tweak wording before posting; this document is the readable copy.
