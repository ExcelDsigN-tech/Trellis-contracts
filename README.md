<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="brand/png/trellis-lockup-dark-320.png">
    <img src="brand/png/trellis-lockup-320.png" alt="Trellis" width="331">
  </picture>
</p>

<h1 align="center">Trellis Smart Contracts</h1>

<p align="center">
  <strong>Open-source Soroban smart contracts powering transparent, secure and verifiable<br>humanitarian aid distribution on the Stellar blockchain.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-1C6B55?style=flat-square" alt="License">
  <img src="https://img.shields.io/badge/Rust-1.80%2B-14201C?style=flat-square" alt="Rust">
  <img src="https://img.shields.io/badge/Soroban-latest-1C6B55?style=flat-square" alt="Soroban">
  <img src="https://img.shields.io/badge/Stellar-blockchain-14201C?style=flat-square" alt="Stellar">
  <img src="https://img.shields.io/badge/status-active%20development-E39A3C?style=flat-square" alt="Status">
</p>

---

# Table of Contents

- Overview
- Why Smart Contracts
- Core Features
- Architecture
- Contract Modules
- Technology Stack
- Project Structure
- Development Setup
- Build
- Testing
- Deployment
- Security
- Upgrade Strategy
- Contract Interfaces
- Events
- Storage Layout
- Contribution Guide
- Brand
- License

---

# Overview

The **Trellis Contracts** repository contains every on-chain component responsible for securely executing humanitarian aid transactions on the Stellar blockchain using **Soroban**.

These contracts serve as the trust layer of the platform.

Instead of relying on centralized intermediaries, every payment, verification, referral reward, and settlement is executed transparently on-chain.

The contracts are designed to be:

- Secure
- Auditable
- Upgradeable
- Gas Efficient
- Modular
- Production Ready

---

# Why Smart Contracts?

Traditional donation systems rely heavily on centralized infrastructure.

Trellis replaces this model by executing critical operations directly on-chain.

The contracts provide:

- Transparent aid settlements
- Immutable transaction records
- Referral commission distribution
- Treasury management
- Multi-signature administration
- Emergency controls
- Upgrade governance

Every important financial action becomes publicly verifiable.

---

# Core Features

## Direct Aid Settlement

Transfers donor funds directly to verified recipients.

Features

- Atomic transfers
- Escrow support
- Expiration handling
- Claim verification
- Replay protection

---

## Claim Links

Generate secure claim identifiers.

Supports

- One-time claims
- Expiration dates
- Maximum claims
- Signature validation
- Hash verification

---

## Referral Rewards

Automatically distributes affiliate commissions.

Supports

- Multi-tier referrals
- Percentage configuration
- Reward limits
- Treasury payouts

---

## Treasury

Central treasury responsible for:

- Holding protocol reserves
- Reward distribution
- Administrative transfers
- Emergency withdrawals

---

## Governance

Administrative controls include

- Contract upgrades
- Parameter updates
- Treasury permissions
- Emergency pause
- Role assignments

---

## Identity Verification

Stores verification references without exposing private user information.

Supports

- Hash verification
- Metadata pointers
- AI verification references
- Off-chain oracle integration

---

# Smart Contract Architecture

```

┌────────────────────────────┐
│        Frontend            │
└────────────┬───────────────┘
│
▼
┌────────────────────────────┐
│       Backend API          │
└────────────┬───────────────┘
│
▼
┌────────────────────────────┐
│      Soroban Contracts     │
├────────────────────────────┤
│ Aid Contract               │
│ Treasury Contract          │
│ Referral Contract          │
│ Governance Contract        │
│ Oracle Contract            │
│ Registry Contract          │
└────────────┬───────────────┘
│
▼
Stellar Ledger

```

---

# Contract Modules

## aid-contract

Responsible for

- Aid creation
- Aid claiming
- Settlement
- Escrow
- Refunds

---

## treasury-contract

Responsible for

- Treasury balances
- Reward distribution
- Protocol funds
- Emergency reserve

---

## referral-contract

Responsible for

- Referral registration
- Commission calculations
- Tier rewards
- Reward claims

---

## governance-contract

Responsible for

- Upgrade authorization
- Admin roles
- Parameter management
- Contract registry

---

## oracle-contract

Responsible for

- AI verification references
- External signatures
- Verification proofs

---

## registry-contract

Responsible for

- Contract discovery
- Address registry
- Version tracking

---

# Technology Stack

| Technology | Purpose |
|------------|---------|
| Rust | Smart contract language |
| Soroban SDK | Contract development |
| Stellar CLI | Deployment |
| Cargo | Package manager |
| Soroban RPC | Network interaction |
| GitHub Actions | CI/CD |

---

# Project Structure

```

contracts/

├── aid-contract/
├── treasury-contract/
├── referral-contract/
├── governance-contract/
├── oracle-contract/
├── registry-contract/
│
├── shared/
│ ├── errors.rs
│ ├── events.rs
│ ├── storage.rs
│ ├── auth.rs
│ ├── math.rs
│ └── utils.rs
│
├── scripts/
│ ├── deploy.sh
│ ├── upgrade.sh
│ ├── initialize.sh
│ └── verify.sh
│
├── tests/
├── Cargo.toml
├── Cargo.lock
└── README.md

```

---

# Development Setup

## Install Rust

```
rustup update
```

Install Soroban CLI

```
cargo install --locked soroban-cli
```

Clone repository

```
git clone https://github.com/TRELLIS-STELLAR/Trellis.git

cd Trellis
```

---

# Build

```
cargo build --release
```

Build optimized WASM

```
cargo build \
--target wasm32v1-none \
--release
```

---

# Testing

Run all tests

```
cargo test
```

Run integration tests

```
cargo test --test integration
```

Generate coverage

```
cargo llvm-cov
```

---

# Deployment

Deploy to Testnet

```
soroban contract deploy \
--wasm target/wasm32v1-none/release/aid_contract.wasm \
--source admin
```

Initialize

```
soroban contract invoke \
--id CONTRACT_ID \
-- initialize
```

---

# Security

The contracts implement

- Reentrancy protection
- Signature verification
- Overflow-safe arithmetic
- Access control
- Replay protection
- Input validation
- Treasury limits
- Emergency pause
- Time-based expirations
- Storage validation

---

# Upgrade Strategy

Supports controlled upgrades through governance.

Only authorized administrators may:

- Upgrade contracts
- Register new implementations
- Pause protocol
- Resume protocol
- Update treasury
- Modify protocol parameters

---

# Storage Layout

Persistent storage includes

Aid Records

- Aid ID
- Donor
- Recipient
- Amount
- Status
- Timestamp

Referral Records

- Wallet
- Referrer
- Commission
- Tier

Treasury

- Balance
- Rewards
- Fees

Governance

- Admins
- Roles
- Versions
- Registry

---

# Events

Contracts emit events for:

AidCreated

AidClaimed

AidSettled

AidRefunded

CommissionPaid

TreasuryDeposit

TreasuryWithdrawal

ContractPaused

ContractResumed

ContractUpgraded

ModuleInitialized

ActionExecuted

PermissionChanged

Off-chain indexers should match the stable two-part topic tuples and decode the typed payloads documented in `shared/README.md`.

---

# Future Roadmap

- Cross-chain settlement
- Stellar Asset support
- Multi-token donations
- DAO governance
- Zero Knowledge verification
- On-chain reputation
- Human identity proofs
- Streaming donations
- Batch settlements

---

# Contributing

We welcome contributions from Rust and Soroban developers.

Read [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide: local setup, build and test instructions, branch and commit conventions, and what reviewers look for.

Please note that this project is released with a [Contributor Code of Conduct](CODE_OF_CONDUCT.md). By participating you agree to abide by its terms.

Workflow

1. Fork repository

2. Create feature branch

3. Write tests

4. Submit Pull Request

Every contract contribution must include:

- Unit tests

- Documentation

- Security considerations

- Gas optimization review

---

# Brand

The Trellis mark, wordmark and palette live in [`brand/`](brand/) — SVG, PNG and ICO
variants for light and dark backgrounds, plus clear-space and minimum-size rules.

| Vine | Amber | Ink | Paper |
| --- | --- | --- | --- |
| `#1C6B55` | `#E39A3C` | `#14201C` | `#F7F5F0` |

Vine is the structure, amber is the accent — one amber element per composition.

---

# License

Licensed under the MIT License.

See LICENSE for details.

---

# Built With

- Stellar
- Soroban
- Rust
- Open Source Community

Building transparent humanitarian infrastructure for everyone.
