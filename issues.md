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

- **`revoke_issuer_proofs` is a stub.** It currently always returns `0`
  and does nothing (`reputation-registry/contracts/reputation-registry/src/reputation_registry.rs`).
  A full implementation needs a secondary index mapping each issuer to the
  proof hashes it issued, which isn't maintained yet. Batch-revoking all of
  a bad issuer's proofs today means walking `get_all_issuers` /
  `get_user_proofs` off-chain and calling `revoke_proof` per proof.

That's the current list. Everything else previously tracked here
(doc comments, event emissions, the leaderboard, score-cap tests, dispute
deadline enforcement, provider stats, etc.) is implemented and tested in
the current codebase — see each contract's `test.rs` for coverage.
