# Security Policy

Trellis contracts hold and move humanitarian aid funds on a public ledger. If you
find a vulnerability, please report it privately so it can be fixed before it is
exploited.

## Audit status

**These contracts have not had a third-party security audit and are deployed
to Stellar testnet only.** Do not deploy them to mainnet or use them with real
funds. The only automated checks are described in
[security/README.md](security/README.md). They are not a substitute for an audit.

## Supported versions

| Version | Supported |
|---|---|
| `main` (latest commit) | Yes |
| Any other branch, tag or fork | No |

There are no tagged releases yet. Fixes land on `main` only.

## Reporting a vulnerability

**Do not open a public issue, pull request or discussion for a security bug.**

Report it through GitHub Private Vulnerability Reporting:
**[Report a vulnerability](https://github.com/TRELLIS-STELLAR/Trellis-contracts/security/advisories/new)**
(Security tab → "Report a vulnerability").

Please include:

- affected crate(s) (for example `treasury-contract`, `shared`)
- commit hash you tested against
- network and contract ID, if reproduced on-chain
- impact: what an attacker can do, and to whose funds
- a proof of concept: a failing test or transaction steps

### What to expect

| Step | Time |
|---|---|
| First response | within **3 business days** |
| Triage and severity assessment | within **7 business days** |
| Fix or mitigation plan for high/critical issues | within **30 days** |

We will keep you updated in the advisory thread, credit you in the published
advisory unless you ask us not to, and coordinate the disclosure date with you.
Please give us 90 days, or until a fix ships, whichever comes first, before
disclosing publicly.

## Scope

In scope: everything in `contracts/` and `shared/`, the deploy/upgrade scripts
in `scripts/`, and the CI supply chain in `.github/workflows/`.

Out of scope: third-party dependencies themselves (report those upstream to the
maintainer or [RustSec](https://rustsec.org/)), and anything already recorded in
[security/audit-allowlist.toml](security/audit-allowlist.toml).

## Dependency advisories

The `audit` job in [`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs
[`scripts/security/run-audit.sh`](scripts/security/run-audit.sh) (`cargo audit`
plus a clippy security preset) on every push, every pull request, and weekly on
Monday at 06:00 UTC. The job **blocks merges**: any RustSec vulnerability in
`Cargo.lock`, or any panic-family lint in contract code, fails the build unless
it has a valid allowlist entry.

When the audit job fails:

1. **Read the advisory** (`RUSTSEC-YYYY-NNNN`) and check whether the vulnerable
   code path is reachable from contract code or is only used by build/test tooling.
2. **Fix it if you can.** Run `cargo update -p <crate>` to pick up a patched
   version, or replace the dependency.
3. **Otherwise, accept it explicitly.** Add an `[[exceptions]]` entry to
   [security/audit-allowlist.toml](security/audit-allowlist.toml) with a written
   `rationale` and an `expires` date of no more than 6 months, and open a tracking
   issue. Entries without a rationale, or past their expiry date, stop suppressing
   the finding. Each entry matches one advisory ID only, so it never hides a new
   advisory.
4. **If the scheduled run fails on `main`,** treat it as a new report: triage it
   within 7 business days, as above.

## Further reading

- [security/README.md](security/README.md): what the automated audit checks and how to run it locally
- [SECURITY_BATCH.md](SECURITY_BATCH.md): security review of the batch operations module
- [UPGRADEABILITY.md](UPGRADEABILITY.md): upgrade safety checklist
