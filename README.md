<div align="center">

# 🔐 RepuZK

### Privacy-Preserving Portable Reputation on Stellar

**Prove your trustworthiness. Reveal nothing.**

[![Built on Stellar](https://img.shields.io/badge/Built%20on-Stellar-7B4AE2?style=flat-square&logo=stellar)](https://stellar.org)
[![Soroban](https://img.shields.io/badge/Smart%20Contracts-Soroban-FF6B35?style=flat-square)](https://soroban.stellar.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg?style=flat-square)](LICENSE)
[![ZK Proofs](https://img.shields.io/badge/ZK-Groth16%20%2F%20Barretenberg-blue?style=flat-square)](#)

</div>

---

## What is RepuZK?

RepuZK is a decentralized reputation protocol that lets anyone **prove their track record without exposing it**.

A freelancer can say *"my success rate exceeds 95%"* — and prove it cryptographically — without revealing who their clients are, how much they earned, or anything else. A developer can prove *"I've completed 20 audits"* without linking to their private repos. A borrower can prove *"my reputation score is above 800"* without opening their financial history.

The trust is real. The data stays private.

---

## The Problem

Reputation today is broken in three ways:

**Centralized** — Upwork, Fiverr, GitHub, and Airbnb own your reputation. You built it, but they hold it.

**Non-portable** — A freelancer with 500 completed jobs starts from zero the moment they move to a new platform. Years of work, gone.

**Privacy-invasive** — Verifying trustworthiness means exposing reviews, earnings, work history, and personal information to strangers.

---

## The Solution

RepuZK decouples reputation from platforms:

- Trusted **issuers** (platforms, DAOs, institutions) sign verifiable credentials off-chain
- Users generate **ZK proofs** attesting to claims — without revealing the raw data
- **Soroban smart contracts** anchor proof records permanently on Stellar
- A **marketplace** lets clients hire verified providers without either party losing privacy

Your reputation becomes a portable, cryptographic asset you own.

---

## How It Works

```
 You (User)
    │
    │  1. Platform issues you a signed credential (off-chain, encrypted)
    │     e.g. { jobs: 250, success_rate: 98, disputes: 0 }
    │
    ▼
 ZK Proof Generator
    │
    │  2. You generate a proof of a claim — not the data itself
    │     e.g. "success_rate > 95" → { proof: "0xabc...", public_signal: true }
    │
    ▼
 Soroban Smart Contracts (Stellar)
    │
    │  3. Proof hash is registered on-chain. Issuer validity is checked.
    │     Your score is computed. Badges are awarded.
    │
    ▼
 Marketplace / Verifier
    │
    │  4. Anyone can verify the claim on-chain. No raw data ever leaves your hands.
    │
    ▼
 Client hires you. Lender approves you. DAO grants you access.
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
│  │                  │      │  (score + proofs)    │ │
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

## Organization Repositories

RepuZK is split across three focused repositories:

| Repository | Description | Stack |
|---|---|---|
| 🔗 [`RepuZK-contract`](https://github.com/RepuZK/RepuZK-contract) | **This repo.** Soroban smart contracts for issuer management, proof storage, scoring, and the marketplace | Rust, Soroban |
| ⚙️ [`RepuZK-backend`](https://github.com/RepuZK/RepuZK-backend) | ZK circuit execution, credential ingestion, proof generation API, issuer service | NestJS, Circom/Noir, PostgreSQL, Redis |
| 🖥️ [`RepuZK-frontend`](https://github.com/RepuZK/RepuZK-frontend) | User dashboard, proof generator UI, marketplace interface, issuer panel | Next.js, TypeScript, Tailwind CSS |

---

## Smart Contracts (This Repo)

### 1. Issuer Registry — `issuer-registry`

> The trust anchor. Only credentials from registered issuers are accepted by the protocol.

| Function | Description |
|---|---|
| `initialize(admin)` | Set the contract admin |
| `add_issuer(address, name, description)` | Register a trusted issuer |
| `remove_issuer(address)` | Deactivate an issuer |
| `update_issuer_status(address, is_active)` | Toggle issuer active state |
| `register_credential_type(issuer, id, name, schema, requires_zk)` | Define a credential schema |
| `issue_credential(issuer, recipient, type_id, hash)` | Record a credential issuance on-chain |
| `is_issuer(address)` | Check if an address is a valid active issuer |
| `get_all_issuers()` / `get_active_issuers()` | List all or active issuers |

**Core types:** `Issuer { address, name, description, is_active, registered_at }` · `CredentialType { id, name, schema, requires_zk }`

---

### 2. Reputation Registry — `reputation-registry`

> The proof store. Manages ZK proof records, computes reputation scores, handles badges and verifications. Cross-contract calls into Issuer Registry to validate every proof source.

| Function | Description |
|---|---|
| `initialize(admin, issuer_registry)` | Set admin and link to issuer registry |
| `register_proof(owner, issuer, proof_hash, credential_hash, credential_type, expires_at, metadata_uri)` | Anchor a new ZK proof on-chain |
| `revoke_proof(owner, proof_hash)` | Invalidate a proof |
| `get_proof(owner, proof_hash)` | Fetch a specific proof record |
| `get_active_proofs(owner)` | List all valid proofs for a user |
| `get_score(owner)` | Get a full `ReputationScore` breakdown |
| `get_score_value(owner)` | Get raw score integer (0–1000) |
| `verify_score_threshold(owner, threshold)` | Boolean — does user meet a score floor? |
| `has_credential(owner, credential_type)` | Check for a specific credential type |
| `request_verification(requester, target, proof_hash)` | Open a verification request |
| `complete_verification(verifier, request_id)` | Close a verification request |
| `create_badge(badge_id, name, description, score_threshold, required_credentials)` | Define an achievement badge |
| `award_badge(owner, badge_id)` | Award a badge to a qualifying user |

**Core types:** `ReputationProof` · `ReputationScore { score, components, proof_count }` · `VerificationRequest` · `ReputationBadge`

---

### 3. Marketplace — `marketplace`

> The economy layer. Reputation-gated service listings with native escrow, dispute resolution, and on-chain feedback.

| Function | Description |
|---|---|
| `initialize(admin, reputation_registry, token, fee_bps)` | Bootstrap the marketplace |
| `create_listing(provider, title, description, category, price, token, min_reputation, required_credentials, delivery_days)` | Post a service listing |
| `update_listing(provider, listing_id, ...)` | Edit an existing listing |
| `purchase_service(buyer, listing_id, payment_tx_hash)` | Purchase a service — funds held in escrow |
| `complete_order(seller, order_id)` | Mark order as delivered |
| `confirm_delivery(buyer, order_id)` | Release escrow to seller |
| `raise_dispute(party, order_id, reason)` | Open a dispute |
| `resolve_dispute(admin, order_id, favor_buyer)` | Admin adjudicates dispute |
| `leave_feedback(reviewer, order_id, rating, comment, completion_proof)` | Submit post-order rating |
| `verify_reputation(user)` | On-chain reputation snapshot for a user |
| `get_listings_by_category(category)` | Browse listings by category |

**Core types:** `Listing` · `Order { status, amount, buyer, seller, delivery_deadline }` · `Feedback` · `ReputationVerification` · `OrderStatus { Created, Paid, Delivered, Completed, Disputed, Resolved }`

---

## Reputation Score Model

Scores range from **0 to 1000** and are computed automatically as proofs are registered.

```
Score = Σ(credential_type_weight × proof_count)

Example breakdown:
  250 jobs completed   →  500 pts
  40 governance votes  →  200 pts
  Course completed     →  100 pts
  Verified human       →   50 pts
  ─────────────────────────────
  Total                →  850 / 1000
```

Scores gate marketplace listings, lending decisions, DAO access, and badge awards.

---

## ZK Statements Supported

| Claim | What's proven | What stays private |
|---|---|---|
| `success_rate > 95%` | You're a reliable provider | Exact rate, clients, earnings |
| `jobs_completed > 100` | You have real experience | Client names, project details |
| `reputation_score > 800` | You're highly trusted | Score breakdown, history |
| `disputes = 0` | Clean track record | All transaction data |
| `governance_votes > 50` | Active DAO contributor | Wallet address, voting choices |
| `gpa > 3.5` | Academic achiever | Transcripts, institution |

---

## Use Cases

**Freelancing** — Prove work quality without doxing your client list. Move between platforms without starting over.

**DeFi / Lending** — Access undercollateralized loans by proving reputation score. No financial history exposed.

**DAO Governance** — Gate proposals or voting weight by contribution history without linking wallet identities.

**Hiring** — Developers prove audit count and contributions to recruiters without exposing private repos.

**Education** — Students prove credentials to employers without sharing transcripts.

---

## Repository Structure

```
RepuZK-contract/
├── issuer-registry/
│   └── contracts/issuer-registry/src/
│       ├── issuer_registry.rs    # Core contract
│       └── test.rs
├── reputation-registry/
│   └── contracts/reputation-registry/src/
│       ├── reputation_registry.rs  # Core contract
│       └── test.rs
└── marketplace/
    └── contracts/market/src/
        ├── marketplace.rs         # Core contract
        └── test.rs
```

---

## Getting Started

**Prerequisites:** [Rust](https://rustup.rs/) · [Stellar CLI](https://developers.stellar.org/docs/tools/stellar-cli)

```bash
# Clone the repo
git clone https://github.com/RepuZK/RepuZK-contract
cd RepuZK-contract

# Build all contracts
cd issuer-registry && cargo build --target wasm32-unknown-unknown --release
cd ../reputation-registry && cargo build --target wasm32-unknown-unknown --release
cd ../marketplace && cargo build --target wasm32-unknown-unknown --release

# Run tests
cargo test
```

### Deploy to Testnet

Deploy in order — each contract depends on the one before it.

```bash
# 1. Deploy Issuer Registry
stellar contract deploy \
  --wasm issuer-registry/target/wasm32-unknown-unknown/release/issuer_registry.wasm \
  --network testnet --source <SECRET_KEY>
# → Save: ISSUER_REGISTRY_CONTRACT_ID

# 2. Deploy Reputation Registry (pass issuer registry ID)
stellar contract deploy \
  --wasm reputation-registry/target/wasm32-unknown-unknown/release/reputation_registry.wasm \
  --network testnet --source <SECRET_KEY>
# → Initialize: stellar contract invoke ... -- initialize --admin <ADMIN> --issuer_registry <ISSUER_REGISTRY_CONTRACT_ID>

# 3. Deploy Marketplace (pass reputation registry ID)
stellar contract deploy \
  --wasm marketplace/target/wasm32-unknown-unknown/release/marketplace.wasm \
  --network testnet --source <SECRET_KEY>
# → Initialize: stellar contract invoke ... -- initialize --admin <ADMIN> --reputation_registry <REPUTATION_REGISTRY_CONTRACT_ID> ...
```

---

## Tech Stack

| Layer | Technology |
|---|---|
| Smart Contracts | Soroban (Rust), `#![no_std]` |
| ZK Proofs | Circom + SnarkJS (Groth16) · Noir + Barretenberg |
| Credential Storage | IPFS / Arweave (user-encrypted) |
| Backend API | NestJS, PostgreSQL, Redis |
| Frontend | Next.js, TypeScript, Tailwind CSS |
| Blockchain | Stellar (Testnet + Mainnet) |

---

## Contributing

RepuZK is organized across three repos. Pick the one that matches your skills:

- **Smart contract work** → you're in the right place (`RepuZK-contract`)
- **ZK circuits & proof generation** → [`RepuZK-backend`](https://github.com/RepuZK/RepuZK-backend)
- **UI/UX & frontend** → [`RepuZK-frontend`](https://github.com/RepuZK/RepuZK-frontend)

Open an issue before submitting a large PR. We review all contributions.

---

## License

MIT — build freely, attribute the project.
