# ZK Reputation Marketplace on Stellar

## Project Overview

**Project Name:** RepuZK  
**Category:** ZK + Identity + Marketplace

### Description

A decentralized reputation network built on Stellar where users can prove trustworthiness, work completion rates, contribution history, and skill achievements using zero-knowledge proofs — without revealing personal information, transaction history, or private records.

Instead of exposing sensitive data like exact earnings, client lists, or wallet history, users generate cryptographic proofs that verify claims such as:

- "I completed more than 100 freelance jobs."
- "My success rate is above 95%."
- "I have never been involved in a dispute."
- "My reputation score exceeds 800."

The trust is real. The data stays private.

---

## Problem

Current reputation systems fail in three ways:

### Centralized
Platforms like Upwork, Fiverr, GitHub, and Airbnb own your reputation data. You built it — they hold it. Users cannot transfer reputation between platforms.

### Privacy-Invasive
Verifying trustworthiness forces users to expose reviews, ratings, history, and personal information — data that can be exploited.

### Non-Portable
A user with 1,000 GitHub contributions, 500 Fiverr jobs, and a 5-star rating starts from zero the moment they move to a new platform.

---

## Solution

A Stellar-powered reputation layer where:

- Platforms issue verifiable credentials
- Reputation is stored as attestations
- ZK proofs verify claims without revealing raw data
- Stellar anchors proof records on-chain
- Users own and control their reputation identity

---

## Real Use Cases

| Domain | What's Proved | What Stays Private |
|---|---|---|
| Freelancing | Success rate > 95% | Clients, projects, earnings |
| DAO / Governance | Contributed to 50+ proposals | Wallet history, voting choices |
| Lending | Reputation score > 800 | Financial records |
| Hiring | 1,000 GitHub contributions, 20 audits | Private repositories |
| Education | GPA > 3.5, course completed | Transcripts, institution |

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
│                          └───────────────────────┘  │
└─────────────────────────────────────────────────────┘
        │
        ▼
  Backend API (NestJS)  ◄──►  Frontend (Next.js)
```

---

## Core Components

### 1. User Identity Layer

Every user gets a Stellar wallet (`GABCD123...`) linked to a DID (`did:stellar:abcd123`).

### 2. Credential Issuers

Trusted issuers create signed credentials off-chain:

```json
// Freelance Platform
{ "jobs_completed": 250, "success_rate": 98, "disputes": 0 }

// Education Platform
{ "course_completed": true, "gpa": 3.8 }

// DAO
{ "votes": 150, "proposals": 40 }
```

### 3. Credential Storage

Credentials are stored off-chain (IPFS / Arweave / Ceramic), encrypted with the user's own keys.

### 4. ZK Proof System

Circuits are written in **Circom** or **Noir**. Example:

```
// Circuit input (private)
success_rate = 98

// Constraint
success_rate >= 95

// Output (public)
{ "proof": "0xabc...", "public_signal": true }
```

The verifier confirms the statement without ever seeing the underlying score.

---

## Reputation Score Model

Scores range from **0 to 1000**, computed from registered proofs:

```
Score = Σ(credential_type_weight × proof_count)

Example:
  250 jobs completed   →  500 pts
  40 governance votes  →  200 pts
  Course completed     →  100 pts
  Verified Human       →   50 pts
  ─────────────────────────────
  Total                →  850 / 1000
```

---

## ZK Statements Supported

| Proof | Claim |
|---|---|
| Proof of Reputation | Score > 800 |
| Proof of Experience | Jobs completed > 100 |
| Proof of Skill | Owns Certified Developer Badge |
| Proof of Trust | Disputes = 0 |
| Proof of Activity | Active within last 30 days |

---

## Stellar Integration

**Why Stellar:**

- Fast settlement and low fees
- Soroban smart contracts
- Built-in asset support
- Global accessibility

### Soroban Contracts

**Contract 1 — Reputation Registry**

Stores ZK proof records on-chain:

```rust
pub struct ReputationProof {
    owner: Address,
    proof_hash: BytesN<32>,
    timestamp: u64,
}
```

Functions: `register_proof()` · `update_proof()` · `revoke_proof()`

**Contract 2 — Issuer Registry**

Manages approved credential issuers.

Functions: `add_issuer()` · `remove_issuer()` · `is_issuer()`

**Contract 3 — Reputation Marketplace**

Reputation-gated service listings with native escrow.

Functions: `create_listing()` · `verify_reputation()` · `purchase_service()` · `leave_feedback()`

---

## Marketplace Flow

```
1. Freelancer joins and uploads credential
        { jobs: 250, success_rate: 98% }

2. Generate ZK proof
        claim: success_rate > 95%

3. Store proof hash on Stellar

4. Client browses listings → verifies proof on-chain

5. Client hires freelancer — no raw data ever exchanged
```

---

## NFT Achievement Badges

Badges are issued as Stellar Assets and serve as credential inputs for ZK proofs:

- 🏅 Top Developer
- 🔍 Verified Auditor
- 📦 100 Projects Completed
- 🗳 DAO Expert

---

## Anti-Sybil Protection

| Mechanism | Details |
|---|---|
| Human Verification | World ID or KYC provider integration |
| Stake Requirement | Lock 10 XLM to participate |
| Reputation Decay | Inactive users gradually lose score |

---

## Revenue Model

| Stream | Pricing |
|---|---|
| Proof Verification Fee | 0.5 XLM per verification |
| Premium Reputation Reports | 5 USDC per report |
| Issuer Subscription | Monthly fee for credential issuers |
| API Access | Per-call pricing for recruiters / lenders |

---

## Technical Stack

| Layer | Technology |
|---|---|
| Smart Contracts | Soroban (Rust), `#![no_std]` |
| ZK Proofs | Circom + SnarkJS (Groth16) · Noir + Barretenberg |
| Credential Storage | IPFS / Arweave (user-encrypted) |
| Backend | NestJS, PostgreSQL, Redis |
| Frontend | Next.js, TypeScript, Tailwind CSS |
| Blockchain | Stellar (Testnet + Mainnet) |

---

## Repository Structure

```
RepuZK-contract/          # Soroban smart contracts (this repo)
├── issuer-registry/
├── reputation-registry/
└── marketplace/

RepuZK-backend/           # ZK circuits, proof generation, credential API
├── proof-generator/
├── proof-verifier/
├── credential-service/
├── issuer-service/
├── api/
└── database/

RepuZK-frontend/          # User dashboard, marketplace UI, issuer panel
├── dashboard/
├── reputation-profile/
├── proof-generator/
├── marketplace/
├── issuer-panel/
└── analytics/
```

---

## Why This Matters

Most reputation systems are either public and non-private, or private but centralized. RepuZK combines Soroban smart contracts, verifiable credentials, and zero-knowledge proofs to create a privacy-preserving reputation economy on Stellar — where trust becomes a portable, transferable asset without sacrificing user privacy.

---

## Grant Pitch

> RepuZK is a decentralized reputation layer for Stellar that lets users prove achievements, trust scores, work history, and credentials using zero-knowledge proofs — without revealing private data. By combining Soroban smart contracts, verifiable credentials, and ZK circuits, the platform enables portable, privacy-preserving reputation across freelancing, lending, hiring, education, and DAO ecosystems.
