# RepuZK Contracts — Contributor Issues

> **Stack:** Rust · Soroban SDK · `#![no_std]`  
> **Contracts:** `issuer-registry` · `reputation-registry` · `marketplace`  
> **Start here:** [`README.md`](./README.md) · [`structure.md`](./structure.md)  
> **Deploy order:** `issuer-registry` → `reputation-registry` → `marketplace`

---

## Labels
| | |
|---|---|
| 🟢 Good First Issue | Small, self-contained — great starting point |
| 🟡 Intermediate | Requires understanding one contract |
| 🔴 Advanced | Touches cross-contract calls or complex logic |
| 🐛 Bug | Something is broken |
| ✨ Feature | New functionality |
| 🧹 Refactor / Chore | Cleanup, no logic change |
| 📄 Docs | Documentation only |
| 🧪 Tests | Testing improvements |

---

## Open Issues

---

### Issuer Registry (`issuer-registry/`)

---

#### #1 · 🟢 📄 Add inline doc comments to all public functions
Every public function in `issuer_registry.rs` lacks a `///` doc comment.

**Done when:** Every `pub fn` has a `///` comment explaining purpose, panics, and auth requirement. No logic changes.

---

#### #2 · 🟢 🧹 Add a `get_issuer_count()` view function
Callers cannot query how many issuers are registered without fetching the full list.

**Done when:** A `get_issuer_count() -> u32` function reads `IssuerCount` from storage and returns it. Includes a unit test.

---

#### #3 · 🟢 🧪 Add a test for `update_issuer_status` toggling active → inactive → active
The existing test suite does not cover the toggle lifecycle.

**Done when:** A test verifies that `is_issuer()` returns `true` → `false` → `true` as status is toggled, and `get_active_issuers()` is consistent at each step.

---

#### #4 · 🟡 ✨ Emit a `("issuer", "add")` event from `add_issuer`
No event is emitted when a new issuer is registered, making off-chain indexing impossible.

**Done when:** `add_issuer` emits `env.events().publish(("issuer", "add"), (addr, name, ts))`. A test asserts the event is present in the sandbox output.

---

#### #5 · 🟡 ✨ Emit a `("credential", "issue")` event from `issue_credential`
Off-chain indexers cannot track new credential issuances.

**Done when:** `issue_credential` emits `env.events().publish(("credential", "issue"), (issuer, user, cred_id, hash, ts))`. A test asserts the event.

---

#### #6 · 🟡 🧪 Add fuzz / property tests for `issue_credential` with arbitrary expiry values
No test covers edge cases around `expires_at` (0 = never, past timestamps, far-future timestamps).

**Done when:** Tests cover `expires_at = 0` (no TTL), `expires_at = current_ledger_time - 1` (already expired), and a future timestamp. Each case asserts the correct TTL is set or not set on the storage entry.

---

#### #7 · 🟡 ✨ Add `get_credentials_by_type(credential_type: String) -> Vec<Address>` view
There is no way to query which users hold a specific credential type across all issuers.

**Done when:** The function iterates `AllIssuers`, collects users with a matching credential type, and returns deduplicated addresses. Includes a test with 2 issuers issuing the same type to overlapping user sets.

---

### Reputation Registry (`reputation-registry/`)

---

#### #8 · 🟢 📄 Add inline doc comments to all public functions
Every `pub fn` in `reputation_registry.rs` lacks a `///` doc comment.

**Done when:** Every public function has a `///` comment covering purpose, auth, and panics. No logic changes.

---

#### #9 · 🟢 🧪 Add a test for score capping at 1,000
The score model caps at 1,000 but no test verifies this boundary.

**Done when:** A test registers enough proofs to exceed 1,000 points and asserts `get_reputation_score(user).score == 1000`.

---

#### #10 · 🟢 🧹 Extract score weight constants to named constants
Score weights (`70`, `50`, `45`, etc.) are magic numbers inside the score calculation function.

**Done when:** Each weight is a named `const` (e.g. `WEIGHT_SUCCESS_RATE: u32 = 70`), the calculation function uses the named constants, and no behavior changes.

---

#### #11 · 🟡 🐛 Fix `get_active_user_proofs` returning expired proofs
Proofs with a past `expires_at` are returned by `get_active_user_proofs` if `is_active` is still `true`.

**Done when:** `get_active_user_proofs` filters out proofs where `expires_at > 0 && expires_at < env.ledger().timestamp()`. A test registers a proof with a past expiry and asserts it is excluded.

---

#### #12 · 🟡 ✨ Implement `revoke_proof` score recalculation
After `revoke_proof`, the user's score is not always recalculated correctly when multiple proofs of the same type exist.

**Done when:** After revoking one of two `success_rate` proofs, the score decreases by exactly one weight unit. A test registers 2 `success_rate` proofs, revokes one, and asserts the score difference is exactly `70`.

---

#### #13 · 🟡 ✨ Add `get_leaderboard(limit: u32) -> Vec<(Address, u32)>` view
There is no way to query top reputation holders.

**Done when:** Returns the top `limit` (max 50) users sorted by score descending. Includes a test with 5 users of different scores verifying correct order and truncation.

---

#### #14 · 🟡 🧪 Add tests for `request_verification` and `complete_verification`
The verification request lifecycle has no dedicated tests.

**Done when:** Tests cover: request created → ID returned, complete with `is_valid = true` → stored correctly, complete with `is_valid = false` → stored correctly, and completing a non-existent request ID → panics.

---

#### #15 · 🔴 ✨ Add `expire_proofs(user: Address)` maintenance function
Expired proofs are never cleaned up, leaving stale data in `UserProofs` and inflating storage.

**Done when:** `expire_proofs(user)` iterates the user's proofs, sets `is_active = false` on all expired ones, recalculates the score, and emits a `("proof", "expire")` event for each. Callable by anyone (public). Includes a test.

---

---

### Marketplace (`marketplace/`)

---

#### #16 · 🟢 📄 Add inline doc comments to all public functions
Every `pub fn` in `marketplace.rs` lacks a `///` doc comment.

**Done when:** Every public function has a `///` comment covering purpose, auth, and panics. No logic changes.

---

#### #17 · 🟢 🧪 Add a test for `get_user_rating` with no feedback
`get_user_rating` panics or returns unexpected results when called for a user with zero feedback.

**Done when:** A test calls `get_user_rating` on a fresh address and asserts it returns `(0, 0)` without panicking.

---

#### #18 · 🟡 ✨ Emit a `("order", "complete")` event from `complete_order`
Off-chain indexers cannot detect order completions — only disputes and feedback are observable.

**Done when:** `complete_order` emits `env.events().publish(("order", "complete"), (order_id, seller, buyer, amount, ts))`. A test asserts the event in the sandbox output.

---

#### #19 · 🟡 🐛 Fix `raise_dispute` not enforcing the delivery deadline window
Buyers can raise a dispute immediately after `purchase_service`, before the delivery deadline passes.

**Done when:** `raise_dispute` panics with a clear message if `env.ledger().timestamp() < order.delivery_deadline`. A test verifies the panic fires before the deadline and succeeds after.

---

#### #20 · 🔴 ✨ Add `get_provider_stats(provider: Address)` view
Providers have no way to query their aggregate stats (total orders, completed orders, average rating, total revenue).

**Done when:** `get_provider_stats` returns `{ total_listings, total_orders, completed_orders, disputed_orders, avg_rating, total_revenue: i128 }` computed from existing storage. Includes a test that creates listings, completes orders, and asserts correct aggregates.

---

## How to Contribute

1. Pick an issue and leave a comment so others know you're working on it.
2. Branch: `git checkout -b feat/issue-{number}-short-description`
3. Make your changes and add/update tests.
4. Run `cargo test` inside the relevant contract directory — all tests must pass.
5. Open a PR referencing the issue: `Closes #N`.

PRs are reviewed within 48 hours. Questions? Drop a comment on the issue. 🙌
