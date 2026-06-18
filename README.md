<div align="center">

# 🔐 RepuZK

### Privacy-Preserving Portable Reputation on Stellar

**Prove your trustworthiness. Reveal nothing.**

[![Built on Stellar](https://img.shields.io/badge/Built%20on-Stellar-7B4AE2?style=flat-square&logo=stellar)](https://stellar.org)
[![Soroban](https://img.shields.io/badge/Smart%20Contracts-Soroban-FF6B35?style=flat-square)](https://soroban.stellar.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg?style=flat-square)](LICENSE)
[![ZK Proofs](https://img.shields.io/badge/ZK-Groth16%20%2F%20Barretenberg-blue?style=flat-square)](#)
[![Testnet](https://img.shields.io/badge/Deployed-Testnet-brightgreen?style=flat-square)](#deployed-contracts)

</div>

---

## What is RepuZK?

RepuZK is a decentralized reputation protocol that lets anyone **prove their track record without exposing it**.

A freelancer can say *"my success rate exceeds 95%"* and prove it cryptographically — without revealing clients, earnings, or anything else. A developer can prove *"I've completed 20 audits"* without linking private repos. A borrower can prove *"my score is above 800"* without opening their financial history.

The trust is real. The data stays private.

---

## The Problem

**Centralized** — Upwork, Fiverr, GitHub, and Airbnb own your reputation. You built it, but they hold it.

**Non-portable** — A freelancer with 500 completed jobs starts from zero the moment they switch platforms.

**Privacy-invasive** — Verifying trustworthiness means exposing reviews, earnings, and personal information to strangers.

---

## The Solution

- Trusted **issuers** (platforms, DAOs, institutions) sign verifiable credentials off-chain
- Users generate **ZK proofs** attesting to claims — without revealing the raw data
- **Soroban smart contracts** anchor proof records permanently on Stellar
- A **marketplace** lets clients hire verified providers without either party losing privacy

Your reputation becomes a portable, cryptographic asset you own.

---

## How It Works

```
 Platform
    │  issues signed credential (off-chain, encrypted)
    │  { jobs: 250, success_rate: 98, disputes: 0 }
    ▼
 ZK Proof Generator
    │  generates proof of claim — not the data
    │  "success_rate > 95" → { proof: "0xabc...", public_signal: true }
    ▼
 Soroban Smart Contracts
    │  proof hash registered on-chain
    │  issuer validity checked · score computed · badges awarded
    ▼
 Marketplace / Verifier
    │  anyone verifies the claim on-chain
    │  no raw data ever leaves the user's hands
    ▼
 Client hires. Lender approves. DAO grants access.
```

---

## System Architecture

```
Reputation Sources (Fiverr, GitHub, DAO, University...)
        │
        ▼
Attestation Service  ──────►  Off-chain Credential Storage (IPFS / Arweave)
        │
        ▼
ZK Proof Generator (Circom + SnarkJS  |  Noir + Barretenberg)
        │
        ▼
┌─────────────────────────────────────────────────────┐
│                  Stellar / Soroban                  │
│                                                     │
│  ┌──────────────────┐      ┌──────────────────────┐ │
│  │  Issuer Registry │◄─────│  Reputation Registry │ │
│  └──────────────────┘      └──────────┬───────────┘ │
│                                       │             │
│                          ┌────────────▼──────────┐  │
│                          │      Marketplace      │  │
│                          │  (escrow + disputes)  │  │
│                          └───────────────────────┘  │
└─────────────────────────────────────────────────────┘
        │
        ▼
  Backend API (NestJS)  ◄──►  Frontend (Next.js)
```

---

## Deployed Contracts

> **Network:** Stellar Testnet

| Contract | Address |
|---|---|
| IssuerRegistry | [`CBKPGRVKOSSLZL3CPLHFMQOUKAFR2HJDSVOVKLNCBBZY5RYPNGI3YE6S`](https://lab.stellar.org/r/testnet/contract/CBKPGRVKOSSLZL3CPLHFMQOUKAFR2HJDSVOVKLNCBBZY5RYPNGI3YE6S) |
| ReputationRegistry | [`CA63GY2TWJTKGECG6FPR4ITW4G5PUH3PCGY7P6HY3EC6NM2VSJIATFOK`](https://lab.stellar.org/r/testnet/contract/CA63GY2TWJTKGECG6FPR4ITW4G5PUH3PCGY7P6HY3EC6NM2VSJIATFOK) |
| Marketplace | [`CBCUF26JXDAT64BEOWD5GPH5MNX5OAYW7BUYWSPBVJII5DSO67R6O4RE`](https://lab.stellar.org/r/testnet/contract/CBCUF26JXDAT64BEOWD5GPH5MNX5OAYW7BUYWSPBVJII5DSO67R6O4RE) |

Full transaction hashes and initialization params: [`deployment.md`](./deployment.md)

---

## Organization Repositories

| Repository | Description | Stack |
|---|---|---|
| 🔗 [`RepuZK-contract`](https://github.com/RepuZK/RepuZK-contract) | **This repo.** Soroban smart contracts — issuer management, proof storage, scoring, marketplace | Rust, Soroban |
| ⚙️ [`RepuZK-backend`](https://github.com/RepuZK/RepuZK-backend) | ZK circuit execution, credential ingestion, proof generation API | NestJS, Circom/Noir, PostgreSQL, Redis |
| 🖥️ [`RepuZK-frontend`](https://github.com/RepuZK/RepuZK-frontend) | User dashboard, proof generator UI, marketplace interface | Next.js, TypeScript, Tailwind CSS |

---

## Smart Contracts

### 1. Issuer Registry

The trust anchor. Only credentials from registered issuers are accepted by the protocol.

| Function | Description |
|---|---|
| `initialize(admin)` | Set the contract admin |
| `add_issuer(address, name, description)` | Register a trusted issuer |
| `remove_issuer(address)` | Remove an issuer |
| `update_issuer_status(address, is_active)` | Toggle issuer active state |
| `register_credential_type(issuer, id, name, schema, requires_zk)` | Define a credential schema |
| `issue_credential(issuer, recipient, type_id, hash, expires_at)` | Record a credential issuance |
| `is_issuer(address)` | Returns `true` if address is a registered active issuer |
| `get_all_issuers()` / `get_active_issuers()` | List issuers |
| `transfer_admin(new_admin)` | Hand off admin role |

---

### 2. Reputation Registry

The proof store. Anchors ZK proof hashes, computes scores, manages badges and verifications. Cross-contract calls into Issuer Registry to validate every proof source.

| Function | Description |
|---|---|
| `initialize(admin, issuer_registry)` | Set admin and link to issuer registry |
| `register_proof(owner, issuer, proof_hash, credential_hash, credential_type, expires_at, metadata_uri)` | Anchor a ZK proof on-chain |
| `revoke_proof(proof_hash, revoker)` | Invalidate a proof (owner / issuer / admin) |
| `get_active_user_proofs(user)` | List all valid, non-expired proofs for a user |
| `get_reputation_score(user)` | Full `ReputationScore` with component breakdown |
| `get_score_value(user)` | Raw score `u32` (consumed by Marketplace) |
| `verify_score_threshold(user, threshold)` | Boolean — does user meet a score floor? |
| `has_credential(user, credential_type)` | Check for a specific credential type |
| `request_verification(requester, target, proof_hash)` | Open a verification request |
| `complete_verification(request_id, verifier, is_valid)` | Close a verification request |
| `create_badge(badge_id, name, description, score_threshold, required_credentials)` | Define a badge (admin) |
| `check_and_award_badges(user)` | Evaluate all badges; award qualifying ones |

**Score model** — computed automatically on every proof change, capped at 1,000:

```
success_rate       → +70 pts per proof
jobs_completed     → +50
verified_human     → +50
proposals          → +45
contributions      → +40
course_completed   → +30
other              → +20
```

---

### 3. Marketplace

The economy layer. Reputation-gated service listings with native escrow, dispute resolution, and on-chain feedback.

| Function | Description |
|---|---|
| `initialize(admin, reputation_registry, issuer_registry, fee_bps, fee_recipient)` | Bootstrap the marketplace |
| `create_listing(provider, title, description, category, price, token, min_score, required_credentials, delivery_days)` | Post a service listing |
| `update_listing(provider, listing_id, new_price?, new_is_active?)` | Edit a listing |
| `purchase_service(buyer, listing_id, zk_proof_hash)` | Reputation check → escrow tokens → create order |
| `start_order(seller, order_id)` | Mark order as in progress |
| `complete_order(seller, order_id, completion_proof)` | Deliver → release escrow minus platform fee |
| `raise_dispute(buyer, order_id, reason)` | Open a dispute |
| `resolve_dispute(admin, order_id, release_to_seller)` | Admin adjudicates — pays seller or refunds buyer |
| `leave_feedback(reviewer, order_id, rating, comment, completion_proof)` | Submit rating (1–5, one per order) |
| `get_listings_by_category(category)` | Browse active listings by category |
| `get_user_rating(user)` | Returns `(average_rating, count)` |

**Escrow flow:**
```
purchase  →  tokens locked in contract
complete  →  seller receives (amount - fee), fee_recipient receives fee
dispute   →  admin decides: seller paid or buyer refunded in full
```

---

## ZK Statements Supported

| Claim | Proven | Private |
|---|---|---|
| `success_rate > 95%` | Reliable provider | Exact rate, clients, earnings |
| `jobs_completed > 100` | Real experience | Client names, project details |
| `reputation_score > 800` | Highly trusted | Score breakdown, history |
| `disputes = 0` | Clean track record | All transaction data |
| `governance_votes > 50` | Active DAO contributor | Wallet, voting choices |
| `gpa > 3.5` | Academic achiever | Transcripts, institution |

---

## Repository Structure

```
RepuZK-contract/
├── issuer-registry/
│   └── contracts/issuer-registry/src/
│       ├── lib.rs
│       ├── issuer_registry.rs
│       └── test.rs
├── reputation-registry/
│   └── contracts/reputation-registry/src/
│       ├── lib.rs
│       ├── reputation_registry.rs
│       └── test.rs
├── marketplace/
│   └── contracts/market/src/
│       ├── lib.rs
│       ├── marketplace.rs
│       └── test.rs
├── deployment.md      ← deployed contract addresses + tx hashes
└── structure.md       ← full implementation + backend/frontend guide
```

---

## Getting Started

**Prerequisites:** [Rust](https://rustup.rs/) · [Stellar CLI](https://developers.stellar.org/docs/tools/stellar-cli)

```bash
git clone https://github.com/RepuZK/RepuZK-contract
cd RepuZK-contract

# Build (Rust 1.82+ requires wasm32v1-none via stellar contract build)
cd issuer-registry && stellar contract build
cd ../reputation-registry && stellar contract build
cd ../marketplace && stellar contract build

# Run tests
cd issuer-registry && cargo test
cd ../reputation-registry && cargo test
cd ../marketplace && cargo test
```

> **Note:** Use `stellar contract build` — not `cargo build --target wasm32-unknown-unknown`. Rust 1.82+ requires the `wasm32v1-none` target, which the Stellar CLI handles automatically.

### Deploy to Testnet

```bash
# 1. Deploy & initialize IssuerRegistry
stellar contract deploy \
  --wasm issuer-registry/target/wasm32v1-none/release/issuer_registry.wasm \
  --source <SECRET_KEY> --network testnet

stellar contract invoke --id <ISSUER_ID> --source <SECRET_KEY> --network testnet \
  -- initialize --admin <ADMIN_ADDRESS>

# 2. Deploy & initialize ReputationRegistry
stellar contract deploy \
  --wasm reputation-registry/target/wasm32v1-none/release/reputation_registry.wasm \
  --source <SECRET_KEY> --network testnet

stellar contract invoke --id <REP_ID> --source <SECRET_KEY> --network testnet \
  -- initialize --admin <ADMIN_ADDRESS> --issuer_registry <ISSUER_ID>

# 3. Deploy & initialize Marketplace
stellar contract deploy \
  --wasm marketplace/target/wasm32v1-none/release/market.wasm \
  --source <SECRET_KEY> --network testnet

stellar contract invoke --id <MKT_ID> --source <SECRET_KEY> --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --reputation_registry <REP_ID> \
  --issuer_registry <ISSUER_ID> \
  --platform_fee_bps 250 \
  --fee_recipient <ADMIN_ADDRESS>
```

---

## Tech Stack

| Layer | Technology |
|---|---|
| Smart Contracts | Soroban (Rust), `#![no_std]`, `wasm32v1-none` |
| ZK Proofs | Circom + SnarkJS (Groth16) · Noir + Barretenberg |
| Credential Storage | IPFS / Arweave (user-encrypted) |
| Backend | NestJS, PostgreSQL, Redis |
| Frontend | Next.js, TypeScript, Tailwind CSS |
| Blockchain | Stellar Testnet / Mainnet |

---

## Contributing

RepuZK is organized across three repos — pick the one matching your skills:

- **Smart contracts** → you're here (`RepuZK-contract`)
- **ZK circuits & proof generation** → [`RepuZK-backend`](https://github.com/RepuZK/RepuZK-backend)
- **UI/UX & frontend** → [`RepuZK-frontend`](https://github.com/RepuZK/RepuZK-frontend)

Open an issue before submitting a large PR.

---

## License

MIT — build freely, attribute the project.
