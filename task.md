#14 · Add tests for request_verification and complete_verification
 #14
Open
Description
@yahia008
yahia008
opened 1w ago
Contributor
The verification request lifecycle has no dedicated tests.

Done when: Tests cover:

Request created → ID returned
Complete with is_valid = true → stored correctly
Complete with is_valid = false → stored correctly
Completing a non-existent request ID → panics
Contract: reputation-registry/

#13 · Add get_leaderboard(limit: u32) -> Vec<(Address, u32)> view
 #13
Open
Description
@yahia008
yahia008
opened 1w ago
Contributor
There is no way to query top reputation holders.

Done when: Returns the top limit (max 50) users sorted by score descending. Includes a test with 5 users of different scores verifying correct order and truncation.

Contract: reputation-registry/

#12 · Fix revoke_proof score recalculation for multiple proofs of same type
 #12
Open
Description
@yahia008
yahia008
opened 1w ago
Contributor
After revoke_proof, the user's score is not always recalculated correctly when multiple proofs of the same type exist.

Done when: After revoking one of two success_rate proofs, the score decreases by exactly one weight unit. A test registers 2 success_rate proofs, revokes one, and asserts the score difference is exactly 70.

Contract: reputation-registry/