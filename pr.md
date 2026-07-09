# feat: verification tests, leaderboard, and revoke score fix

Closes #12 · Closes #13 · Closes #14

---

## Summary

This PR addresses three open issues in the `reputation-registry` contract, covering a bug fix, a new view function, and expanded test coverage.

---

## Changes

### #14 · Tests for `request_verification` and `complete_verification`

The verification request lifecycle had no dedicated tests. Added five focused tests:

| Test | What it covers |
|---|---|
| `test_verification_request_returns_id` | First request returns ID `0` |
| `test_complete_verification_is_valid_true` | `is_verified=true` stored correctly, all fields match |
| `test_complete_verification_is_valid_false` | `is_verified=false` stored correctly |
| `test_complete_verification_nonexistent_request` | Panics with `"request not found"` on unknown ID |
| `test_multiple_verification_requests_sequential_ids` | IDs increment; completing one leaves the other pending |

---

### #13 · `get_leaderboard(limit: u32) -> Vec<(Address, u32)>`

Replaces the `get_top_users` stub (which returned an empty vec) with a real on-chain implementation.

**How it works:**
- `register_proof` now tracks each new user in an `AllUsers` storage key on their first proof registration.
- `get_leaderboard` reads all users, pairs each with their current score, insertion-sorts descending, and truncates to `min(limit, 50)`.

**Tests:**
- `test_get_leaderboard_correct_order_and_truncation` — 5 users with distinct scores (`120, 100, 70, 50, 40`); verifies sort order, correct addresses at top positions, and truncation to `limit=3`
- `test_get_leaderboard_cap_at_50` — calling with `limit=100` on 5 users returns all 5 without error

---

### #12 · Score recalculation after revoking one of multiple same-type proofs

The existing `revoke_proof` sets `is_active=false` in storage before calling `update_reputation_score`, which re-sums only active proofs via `get_active_user_proofs`. The recalculation logic was correct but untested for the multi-proof case.

**Tests:**
- `test_revoke_one_of_two_same_type_proofs_score_diff_is_70` — registers 2 `success_rate` proofs (`score=140`), revokes one, asserts score drops to `70` (difference = exactly `70`)
- `test_revoke_all_proofs_zeroes_score` — revokes both proofs step by step and asserts score reaches `0`

---

## Files changed

```
reputation-registry/contracts/reputation-registry/src/
├── reputation_registry.rs   ← AllUsers key, user tracking, get_leaderboard
└── test.rs                  ← 9 new tests across #12, #13, #14
```

---

## Testing

```bash
cd reputation-registry && cargo test
```

All existing tests continue to pass. 9 new tests added.

> **Note:** `cargo` / Rust toolchain must be installed locally to run tests. See [Getting Started](../README.md#getting-started).
