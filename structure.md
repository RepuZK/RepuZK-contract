# RepuZK — Full System Structure

This document describes how every layer of RepuZK is implemented: the three Soroban smart contracts, the NestJS backend, and the Next.js frontend.

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Frontend (Next.js)                       │
│  Dashboard │ Proof Generator │ Marketplace │ Issuer Panel       │
└────────────────────────────┬────────────────────────────────────┘
                             │ REST / WebSocket
┌────────────────────────────▼────────────────────────────────────┐
│                       Backend (NestJS)                          │
│  Credential API │ ZK Proof Service │ Issuer Service │ Auth      │
└──────────┬───────────────────────────────────────┬─────────────┘
           │ PostgreSQL / Redis                    │ Stellar SDK
┌──────────▼──────────┐              ┌─────────────▼─────────────┐
│  Off-chain Storage  │              │    Stellar / Soroban       │
│  PostgreSQL · Redis │              │  IssuerRegistry            │
│  IPFS / Arweave     │              │  ReputationRegistry        │
└─────────────────────┘              │  Marketplace               │
                                     └───────────────────────────-┘
```

---

## Part 1 — Smart Contracts

### Contract Dependency Graph

```
IssuerRegistry
    ▲
    │  cross-contract: is_issuer()
ReputationRegistry
    ▲
    │  cross-contract: get_score_value(), has_credential()
Marketplace
```

Deploy in order: IssuerRegistry → ReputationRegistry → Marketplace.

---

### 1.1 Issuer Registry

**Source:** `issuer-registry/contracts/issuer-registry/src/issuer_registry.rs`

#### Data Structures

```rust
Issuer { address, name, description, is_active, registered_at, updated_at }
CredentialType { id, name, description, schema, requires_zk }
```

#### Storage (instance unless noted)

| Key | Value | Notes |
|---|---|---|
| `Admin` | `Address` | |
| `Issuer(Address)` | `Issuer` | |
| `AllIssuers` | `Vec<Address>` | |
| `IssuerCredentialTypes(Address)` | `Vec<CredentialType>` | |
| `IssuerCount` | `u32` | |
| `(issuer, user, credential_id)` tuple | `BytesN<32>` | **persistent**, optional TTL |

#### Public Functions

| Function | Auth | Behaviour |
|---|---|---|
| `initialize(admin)` | admin | Panics if already initialized |
| `add_issuer(addr, name, desc)` | admin | Panics on duplicate |
| `remove_issuer(addr)` | admin | Deletes record, rebuilds list |
| `update_issuer_status(addr, is_active)` | admin | Toggle active flag |
| `register_credential_type(issuer, id, name, desc, schema, requires_zk)` | issuer | Issuer must be active |
| `issue_credential(issuer, user, cred_id, hash, expires_at)` | issuer | Persistent storage; TTL if `expires_at > 0` |
| `is_issuer(addr)` | — | Returns `true` if active |
| `get_issuer(addr)` | — | |
| `get_all_issuers()` / `get_active_issuers()` | — | |
| `verify_credential_type(issuer, cred_id)` | — | |
| `transfer_admin(new_admin)` | admin | |

---

### 1.2 Reputation Registry

**Source:** `reputation-registry/contracts/reputation-registry/src/reputation_registry.rs`

Cross-contract call on every `register_proof`: `IssuerRegistryClient::is_issuer()`.

#### Data Structures

```rust
ReputationProof {
    owner, issuer, proof_hash: BytesN<32>, credential_hash: BytesN<32>,
    credential_type: String, registered_at, expires_at,   // 0 = never
    is_active: bool, metadata_uri: String,
}
ReputationScore {
    score: u32,                  // 0–1000
    calculated_at: u64,
    components: Map<String, u32>,
    proof_count: u32,
}
VerificationRequest { requester, target, proof_hash, requested_at, is_verified, verified_at }
ReputationBadge { badge_id, name, description, score_threshold, required_credentials, is_active }
```

#### Storage (all instance)

| Key | Value |
|---|---|
| `Admin` / `IssuerRegistry` | `Address` |
| `ProofData(BytesN<32>)` | `ReputationProof` |
| `UserProofs(Address)` | `Vec<BytesN<32>>` |
| `UserScore(Address)` | `ReputationScore` |
| `VerificationRequest(u64)` | `VerificationRequest` |
| `ReputationBadge(String)` | `ReputationBadge` |
| `AllBadges` | `Vec<String>` |
| `BadgeHolders(String)` | `Vec<Address>` |
| `UserBadges(Address)` | `Vec<String>` |
| `NextRequestId` / `TotalProofs` / `VerificationCount` | `u64` / `u32` |

#### Public Functions

| Function | Auth | Behaviour |
|---|---|---|
| `initialize(admin, issuer_registry)` | admin | |
| `register_proof(owner, issuer, proof_hash, credential_hash, credential_type, expires_at, metadata_uri)` | owner | Validate issuer → store → recalculate score |
| `revoke_proof(proof_hash, revoker)` | owner / issuer / admin | Sets `is_active = false`, recalculates score |
| `get_proof(hash)` / `get_user_proofs(user)` / `get_active_user_proofs(user)` | — | |
| `get_reputation_score(user)` | — | Full struct |
| `get_score_value(user)` | — | Raw `u32` — consumed by Marketplace |
| `verify_score_threshold(user, threshold)` | — | Boolean |
| `has_credential(user, credential_type)` | — | Checks active proofs |
| `request_verification(requester, target, proof_hash)` | requester | Returns request ID |
| `complete_verification(request_id, verifier, is_valid)` | verifier | |
| `create_badge(badge_id, name, desc, threshold, required_creds)` | admin | |
| `check_and_award_badges(user)` | user | Evaluates all badges, awards qualifying ones |
| `get_user_badges(user)` / `get_all_badges()` | — | |
| `transfer_admin(new_admin)` | admin | |

#### Score Calculation

Recalculated automatically after every proof mutation. Per active, non-expired proof:

```
success_rate       → +70
jobs_completed     → +50
verified_human     → +50
proposals          → +45
contributions      → +40
course_completed   → +30
(other)            → +20

Total capped at 1,000
```

#### Events

| Topics | Data |
|---|---|
| `("proof", "reg")` | `(owner, issuer, proof_hash, credential_type, ts)` |
| `("score", "upd")` | `(user, new_score, ts)` |
| `("badge", "get")` | `(user, badge_id, badge_name, ts)` |

---

### 1.3 Marketplace

**Source:** `marketplace/contracts/market/src/marketplace.rs`

Cross-contract calls: `ReputationRegistryClient::get_score_value()` and `has_credential()` on every purchase.

#### Data Structures

```rust
Listing {
    listing_id, provider, title, description, category,
    price: i128, token_address: Address,
    min_reputation_score: u32, required_credentials: Vec<String>,
    delivery_days: u32, is_active: bool, created_at, updated_at,
}
Order {
    order_id, listing_id, buyer, seller, amount: i128, token_address,
    status: OrderStatus, payment_tx_hash: BytesN<32>,
    created_at, paid_at, completed_at, delivery_deadline,
}
// OrderStatus: Created | Paid | InProgress | Completed | Disputed | Cancelled | Refunded
Feedback { feedback_id, order_id, reviewer, reviewee, rating: u32 (1–5), comment, completion_proof, created_at, is_verified }
ReputationVerification { user, score, credentials, verified_at, proof_hash, is_valid }
```

#### Storage (all instance)

| Key | Value |
|---|---|
| `Admin` / `ReputationRegistry` / `IssuerRegistry` | `Address` |
| `PlatformFeeBps` | `u32` (e.g. 250 = 2.5%) |
| `FeeRecipient` / `MinListingPrice` / `EscrowDurationDays` | config |
| `Listing(u64)` | `Listing` |
| `AllListings` / `ProviderListings(Address)` / `CategoryListings(String)` | `Vec<u64>` |
| `Order(u64)` | `Order` |
| `BuyerOrders(Address)` / `SellerOrders(Address)` | `Vec<u64>` |
| `Feedback(u64)` / `OrderFeedback(u64)` | `Feedback` / `u64` |
| `UserFeedbackReceived(Address)` / `UserFeedbackGiven(Address)` | `Vec<u64>` |
| `ReputationVerification(Address)` | snapshot |
| `NextListingId` / `NextOrderId` / `NextFeedbackId` | `u64` |

#### Public Functions

| Function | Auth | Behaviour |
|---|---|---|
| `initialize(admin, rep_registry, issuer_registry, fee_bps, fee_recipient)` | admin | |
| `create_listing(provider, title, desc, category, price, token, min_score, required_creds, delivery_days)` | provider | Validates price ≥ min and delivery 1–90 days |
| `update_listing(provider, listing_id, new_price?, new_is_active?)` | provider | Partial update |
| `verify_reputation(user, required_score, required_creds, zk_proof_hash)` | user | Cross-contract check → store snapshot |
| `purchase_service(buyer, listing_id, zk_proof_hash)` | buyer | Score/credential check → escrow tokens → create order |
| `start_order(seller, order_id)` | seller | `Paid → InProgress` |
| `complete_order(seller, order_id, completion_proof)` | seller | `InProgress → Completed` → release escrow |
| `raise_dispute(buyer, order_id, reason)` | buyer | `Paid/InProgress → Disputed` |
| `resolve_dispute(admin, order_id, release_to_seller)` | admin | Pay seller or refund buyer |
| `leave_feedback(reviewer, order_id, rating, comment, completion_proof)` | buyer | One per order; order must be Completed |
| `get_listing(id)` / `get_active_listings()` / `get_listings_by_category(cat)` | — | |
| `get_order(id)` / `get_buyer_orders(addr)` / `get_seller_orders(addr)` | — | |
| `get_feedback(id)` / `get_user_feedback_received(addr)` / `get_user_rating(addr)` | — | Returns `(avg, count)` |
| `get_platform_stats()` | — | `(active_listings, fee_bps, min_price)` |

#### Escrow & Fee Flow

```
purchase_service:   token.transfer(buyer → contract, full amount)

complete_order / resolve_dispute (seller wins):
    fee    = amount × fee_bps / 10_000
    seller = amount - fee
    token.transfer(contract → seller, seller)
    token.transfer(contract → fee_recipient, fee)

resolve_dispute (buyer wins):
    token.transfer(contract → buyer, full amount)
```

#### Events

| Topics | Data |
|---|---|
| `("listing", "create")` | `(listing_id, provider, title, price)` |
| `("order", "create")` | `(order_id, listing_id, buyer, seller, price)` |
| `("dispute", "raise")` | `(order_id, buyer, reason)` |
| `("dispute", "resolve")` | `(order_id, release_to_seller)` |
| `("feedback", "submit")` | `(feedback_id, order_id, reviewer, reviewee, rating)` |

---

## Part 2 — Backend (NestJS)

**Repo:** `RepuZK-backend`  
**Stack:** NestJS · TypeScript · PostgreSQL · Redis · Stellar SDK · Circom/SnarkJS or Noir

### Module Structure

```
src/
├── app.module.ts
├── auth/                  # JWT + Stellar wallet auth
│   ├── auth.module.ts
│   ├── auth.service.ts    # challenge/verify signature
│   ├── auth.controller.ts
│   └── jwt.strategy.ts
├── issuer/                # Issuer management
│   ├── issuer.module.ts
│   ├── issuer.service.ts
│   └── issuer.controller.ts
├── credential/            # Credential ingestion + signing
│   ├── credential.module.ts
│   ├── credential.service.ts
│   └── credential.controller.ts
├── proof/                 # ZK proof generation + verification
│   ├── proof.module.ts
│   ├── proof.service.ts
│   ├── proof.controller.ts
│   └── circuits/          # Compiled Circom/Noir circuits
├── reputation/            # Score queries + on-chain sync
│   ├── reputation.module.ts
│   ├── reputation.service.ts
│   └── reputation.controller.ts
├── stellar/               # Soroban contract clients
│   ├── stellar.module.ts
│   └── stellar.service.ts
└── common/
    ├── database/          # TypeORM entities + migrations
    ├── redis/             # Cache + job queue client
    └── guards/            # Auth guards, roles
```

### Database Schema (PostgreSQL)

```sql
-- Registered issuers (mirrors on-chain)
issuers (id, stellar_address, name, description, is_active, created_at)

-- Credential types per issuer
credential_types (id, issuer_id, type_id, name, schema_json, requires_zk)

-- Off-chain credential payloads (before ZK proof is generated)
credentials (
    id, issuer_id, user_address, credential_type,
    payload_json,          -- raw data e.g. { jobs: 250, success_rate: 98 }
    payload_hash,          -- keccak/sha256 of payload
    ipfs_cid,              -- uploaded to IPFS after signing
    issued_at, expires_at
)

-- ZK proof records (mirrors on-chain ProofData)
proofs (
    id, user_address, issuer_address, credential_id,
    proof_hash,            -- BytesN<32> registered on-chain
    proof_json,            -- full proof object (kept off-chain)
    public_signals_json,
    circuit_name,          -- e.g. "success_rate_gt_95"
    stellar_tx_hash,       -- tx that called register_proof
    registered_at, expires_at, is_active
)

-- Verification requests
verifications (id, requester_address, target_address, proof_hash, requested_at, completed_at, is_valid)
```

### Redis Usage

| Key pattern | Purpose |
|---|---|
| `challenge:{address}` | Auth challenge nonce (TTL 5 min) |
| `score:{address}` | Cached reputation score (TTL 60 s) |
| `proof:status:{job_id}` | ZK proof generation job status |

Bull queues:
- `proof-generation` — async ZK proof jobs
- `stellar-submit` — async on-chain transactions

### API Endpoints

#### Auth

```
POST /auth/challenge          { address } → { nonce }
POST /auth/verify             { address, signature, nonce } → { access_token }
```

#### Issuer

```
POST /issuer/register         { name, description } (admin)
POST /issuer/credential-type  { typeId, name, schema, requiresZk }
POST /issuer/issue            { userAddress, credentialType, payload }
GET  /issuer/:address
GET  /issuer/all
```

#### Credential

```
GET  /credential/user/:address
GET  /credential/:id
POST /credential/upload-ipfs   { credentialId } → { cid }
```

#### Proof

```
POST /proof/generate           { credentialId, circuitName, privateInputs } → { jobId }
GET  /proof/status/:jobId      → { status, proofHash? }
POST /proof/register           { proofHash, credentialHash, credentialType, expiresAt, metadataUri }
GET  /proof/user/:address
POST /proof/revoke             { proofHash }
```

#### Reputation

```
GET  /reputation/score/:address         → { score, components, proofCount }
GET  /reputation/verify/:address?threshold=800
GET  /reputation/badges/:address
POST /reputation/verify-on-chain        { userAddress, requiredScore, requiredCredentials, zkProofHash }
```

### ZK Proof Service

The proof service wraps circuit execution. Each claim type maps to a circuit:

```
Circuit name             Claim proved
─────────────────────────────────────
success_rate_gt_N        success_rate >= N
jobs_completed_gt_N      jobs_completed >= N
score_gt_N               reputation_score >= N
disputes_zero            disputes === 0
votes_gt_N               governance_votes >= N
gpa_gt_N                 gpa >= N * 10 (integer arithmetic)
```

**Flow:**

```
1. Frontend sends: { credentialId, circuitName, privateInputs }
2. Backend loads credential payload from DB
3. Compile inputs → run snarkjs.groth16.fullProve(inputs, wasm, zkey)
4. Serialize { proof, publicSignals }
5. Compute proof_hash = sha256(JSON.stringify(proof))
6. Store in DB; push stellar-submit job
7. Stellar job calls reputation_registry.register_proof(...)
8. Return proofHash to frontend
```

### Stellar Service

Wraps `@stellar/stellar-sdk` contract clients generated from contract ABIs:

```typescript
// Calls IssuerRegistry, ReputationRegistry, Marketplace
class StellarService {
  async registerProof(owner, issuer, proofHash, credentialHash, credentialType, expiresAt, metadataUri)
  async getScoreValue(user): Promise<number>
  async hasCredential(user, credentialType): Promise<boolean>
  async createListing(provider, title, category, price, tokenAddress, minScore, requiredCreds, deliveryDays)
  async purchaseService(buyer, listingId, zkProofHash)
  async resolveDispute(admin, orderId, releaseToSeller)
}
```

---

## Part 3 — Frontend (Next.js)

**Repo:** `RepuZK-frontend`  
**Stack:** Next.js 14 (App Router) · TypeScript · Tailwind CSS · Stellar Wallets Kit

### Page & Route Structure

```
app/
├── layout.tsx                 # Root layout: wallet provider, nav
├── page.tsx                   # Landing page
│
├── dashboard/
│   └── page.tsx               # Score overview, active proofs, badges
│
├── proofs/
│   ├── page.tsx               # List all proofs
│   ├── generate/
│   │   └── page.tsx           # Select credential → choose claim → generate ZK proof
│   └── [proofHash]/
│       └── page.tsx           # Proof detail + verification status
│
├── marketplace/
│   ├── page.tsx               # Browse listings (filter by category/score)
│   ├── create/
│   │   └── page.tsx           # Create a service listing
│   ├── [listingId]/
│   │   └── page.tsx           # Listing detail + purchase flow
│   └── orders/
│       └── page.tsx           # Buyer & seller order management
│
├── issuer/
│   ├── page.tsx               # Issuer dashboard (gated to registered issuers)
│   ├── credentials/
│   │   └── page.tsx           # Issue credentials to users
│   └── types/
│       └── page.tsx           # Manage credential types
│
└── verify/
    └── [address]/
        └── page.tsx           # Public reputation profile — verify anyone
```

### Component Structure

```
components/
├── layout/
│   ├── Navbar.tsx
│   └── Sidebar.tsx
├── wallet/
│   ├── WalletConnect.tsx      # Stellar Wallets Kit integration
│   └── WalletButton.tsx
├── reputation/
│   ├── ScoreCard.tsx          # Circular score gauge + breakdown
│   ├── ProofList.tsx          # Table of registered proofs
│   ├── ProofCard.tsx          # Single proof with status badge
│   └── BadgeGrid.tsx          # NFT-style badge display
├── proof/
│   ├── ProofGeneratorWizard.tsx  # Multi-step: select credential → claim → generate → register
│   ├── ClaimSelector.tsx         # Choose circuit/claim type
│   └── ProofStatusPoller.tsx     # Polls /proof/status/:jobId
├── marketplace/
│   ├── ListingGrid.tsx
│   ├── ListingCard.tsx
│   ├── ListingFilters.tsx        # Category, min score, price range
│   ├── PurchaseFlow.tsx          # Reputation check → confirm → escrow
│   ├── OrderCard.tsx
│   └── FeedbackForm.tsx
└── issuer/
    ├── CredentialForm.tsx
    └── CredentialTypeForm.tsx
```

### State Management

Use React Query (`@tanstack/react-query`) for all server state:

```typescript
// Key hooks
useWallet()                          // Stellar wallet connection + address
useReputationScore(address)          // GET /reputation/score/:address
useUserProofs(address)               // GET /proof/user/:address
useUserBadges(address)               // GET /reputation/badges/:address
useListings(category?, minScore?)    // GET marketplace listings
useOrder(orderId)                    // GET /marketplace/orders/:id
useGenerateProof()                   // POST /proof/generate mutation
useRegisterProof()                   // POST /proof/register mutation
usePurchaseService()                 // POST /marketplace/purchase mutation
```

### Wallet Integration

Use `@stellar/stellar-wallets-kit` to support Freighter, xBull, Lobstr, and others:

```typescript
// Auth flow
1. User clicks "Connect Wallet"
2. GET /auth/challenge → { nonce }
3. Wallet signs nonce → signature
4. POST /auth/verify → { access_token }
5. Store JWT in httpOnly cookie
```

### Proof Generator Wizard (key UX flow)

```
Step 1 — Select credential
    Show credentials fetched from GET /credential/user/:address
    User picks e.g. "Fiverr · jobs_completed: 250"

Step 2 — Choose claim
    Given credential type, show supported circuits:
    e.g. "Prove jobs_completed > [N]" — user sets threshold

Step 3 — Generate proof
    POST /proof/generate → { jobId }
    Poll /proof/status/:jobId every 2 s
    Show spinner → "Proof generated ✓"

Step 4 — Register on-chain
    POST /proof/register with returned proofHash
    Show Stellar tx link on success

Step 5 — Done
    Redirect to /dashboard — score updates automatically
```

### Environment Variables

```bash
# Frontend (.env.local)
NEXT_PUBLIC_API_URL=https://api.repuzk.xyz
NEXT_PUBLIC_STELLAR_NETWORK=testnet
NEXT_PUBLIC_ISSUER_REGISTRY_CONTRACT=C...
NEXT_PUBLIC_REPUTATION_REGISTRY_CONTRACT=C...
NEXT_PUBLIC_MARKETPLACE_CONTRACT=C...

# Backend (.env)
DATABASE_URL=postgresql://...
REDIS_URL=redis://...
STELLAR_NETWORK=testnet
STELLAR_ADMIN_SECRET=S...
ISSUER_REGISTRY_CONTRACT=C...
REPUTATION_REGISTRY_CONTRACT=C...
MARKETPLACE_CONTRACT=C...
JWT_SECRET=...
IPFS_API_URL=https://api.pinata.cloud
IPFS_API_KEY=...
```

---

## Part 4 — Data Flow: End-to-End Example

**Scenario:** Freelancer proves `success_rate > 95%` and gets hired.

```
1. ISSUER (Fiverr integration)
   POST /issuer/issue
   { userAddress, credentialType: "success_rate", payload: { rate: 98, jobs: 250 } }
   → Backend stores in DB, uploads encrypted payload to IPFS
   → Calls issuer_registry.issue_credential(issuer, user, "success_rate", hash, 0)

2. USER generates ZK proof
   POST /proof/generate
   { credentialId, circuitName: "success_rate_gt_95", privateInputs: { rate: 98 } }
   → Backend runs snarkjs.groth16.fullProve
   → Returns { jobId }

   GET /proof/status/:jobId  (polled by frontend)
   → { status: "complete", proofHash: "0xabc..." }

3. USER registers proof on-chain
   POST /proof/register
   { proofHash, credentialHash, credentialType: "success_rate", metadataUri: "ipfs://..." }
   → Backend calls reputation_registry.register_proof(...)
   → Score recalculated on-chain (e.g. +70 pts)

4. PROVIDER creates listing
   POST /marketplace/create-listing
   { title: "Smart Contract Audit", minReputationScore: 300, price: 500_0000000, ... }
   → Calls marketplace.create_listing(...)

5. CLIENT purchases service
   POST /marketplace/purchase
   { listingId, zkProofHash }
   → Score check via cross-contract call
   → Tokens escrowed in Marketplace contract
   → Order created with status Paid

6. SELLER completes order
   → marketplace.complete_order(seller, orderId)
   → Escrow released: seller gets amount - fee, fee goes to fee_recipient

7. CLIENT leaves feedback
   → marketplace.leave_feedback(buyer, orderId, 5, "Excellent work", completionProof)
```

---

## Test Coverage (Contracts)

| Contract | Tests |
|---|---|
| IssuerRegistry | initialize, double-init, add/get issuer, duplicate, remove, update status, credential type, issue credential, get all/active, transfer admin |
| ReputationRegistry | initialize, double-init, register/get proof, duplicate proof, revoke, active filter, score increase, threshold, request/complete verification, badge award |
| Marketplace | create listing, price guard, update listing, purchase + escrow, full order lifecycle + payout, dispute + refund, dispute + seller release, leave feedback, listings by category |

Test snapshots: `*/test_snapshots/test/*.json`
