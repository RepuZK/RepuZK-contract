# Known Gaps / Roadmap

RepuZK is maintained solo. This file replaces the previous issue tracker,
which listed #1–#20 as open — in reality 19 of those 20 were already
implemented in the current code; it was just never updated. This is a short,
honest list of what's actually still open.

> **Stack:** Rust · Soroban SDK · `#![no_std]`
> **Contracts:** `issuer-registry` · `reputation-registry` · `marketplace`
> **Start here:** [`README.md`](./README.md) · [`structure.md`](./structure.md)
> **Deploy order:** `issuer-registry` → `reputation-registry` → `marketplace`

---

## Recently closed

- **`revoke_issuer_proofs` was a stub** that always returned `0`
  (`reputation-registry`). Now backed by an `IssuerProofs` index maintained
  in `register_proof`: it walks every proof hash an issuer has issued,
  deactivates the active ones, recalculates each affected owner's score,
  and emits a `("proof", "revoke")` event per proof — the batch equivalent
  of calling `revoke_proof` once per proof. Callable by the contract admin
  or the issuer itself. Covered by
  `test_revoke_issuer_proofs_revokes_only_that_issuers_active_proofs`,
  `test_issuer_can_revoke_its_own_issued_proofs`, and
  `test_revoke_issuer_proofs_by_non_admin_non_issuer_panics`.
- **Single-step `transfer_admin` could permanently brick admin control**
  on a typo'd address (`issuer-registry`, `reputation-registry`). Replaced
  with `propose_admin` / `accept_admin`: the outgoing admin keeps control
  until the proposed address calls `accept_admin` itself. Covered by
  `test_propose_and_accept_admin` and `test_accept_admin_without_proposal_panics`
  in both contracts' `test.rs`.
- **`marketplace::complete_order` / `resolve_dispute` transferred escrowed
  tokens before persisting the order's terminal status** — a
  checks-effects-interactions violation. Both now write `Completed` /
  `Refunded` to storage first, then move funds.
- **`update_listing` silently ignored a `new_price` below
  `MinListingPrice`** instead of rejecting it, so a caller could believe a
  price update took effect when it hadn't. Now panics with `"price below
  minimum"`, consistent with `create_listing`. Covered by
  `test_update_listing_rejects_price_below_minimum`.
- **Removed the dead `get_top_users` stub** (`reputation-registry`) —
  superseded by `get_leaderboard`, which does the same job with a real
  on-chain implementation.
- **`complete_verification` had no verifier authorization check** — any
  address could pass itself in as `verifier` and self-authorize a
  verification request. Fixed: `verifier` must now be the contract admin or
  a registered, active issuer (checked via the same cross-contract call
  into `issuer-registry` that `register_proof` already used). Covered by
  `test_complete_verification_rejects_non_issuer_verifier` and
  `test_complete_verification_allows_registered_active_issuer` in
  `reputation-registry/contracts/reputation-registry/src/test.rs`.
- **`marketplace::initialize` didn't validate `platform_fee_bps`** — a
  value above `10000` (100%) would make `release_to_seller` compute a
  negative `seller_amount` and panic on every `complete_order` /
  `resolve_dispute` call, instead of failing cleanly at setup. Fixed:
  `initialize` now panics with `"fee_bps must be <= 10000"` if the fee
  exceeds 100%. Covered by `test_initialize_rejects_fee_bps_over_100_percent`
  and `test_initialize_accepts_valid_fee_bps` in
  `marketplace/contracts/market/src/test.rs`.
- **`get_active_user_proofs` had no direct expiry test** — the filtering
  logic (`is_active && (expires_at==0 || expires_at>now)`) was already
  correct, but existing tests only exercised revocation for this function,
  or expiry indirectly through `expire_proofs`. Added
  `test_get_active_user_proofs_excludes_expired_proof`, which registers a
  proof with a past `expires_at` (still `is_active = true` in storage) and
  asserts it's excluded from the result directly.

## Open

- **`AllUsers` / `AllListings` / `AllIssuers`-style index vectors grow
  unbounded.** `get_leaderboard`, `get_active_listings`, and
  `get_all_issuers` all do a full scan of their backing index. Cheap to
  write to (still auth-gated), but there's no pagination, so a large
  enough registry makes these reads increasingly expensive. Not urgent at
  current scale; worth revisiting if any of these lists grow into the
  thousands.

That's the current list. Everything else previously tracked here
(doc comments, event emissions, the leaderboard, score-cap tests, dispute
deadline enforcement, provider stats, etc.) is implemented and tested in
the current codebase — see each contract's `test.rs` for coverage.
