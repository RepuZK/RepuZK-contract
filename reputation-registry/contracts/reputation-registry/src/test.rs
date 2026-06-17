#![cfg(test)]

use super::reputation_registry::*;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String, Vec};

// We need the IssuerRegistry to set up the cross-contract call
use soroban_sdk::testutils::Ledger;

/// Register an IssuerRegistry and add an issuer; returns (issuer_registry_id, issuer_addr)
fn setup_issuer_registry(env: &Env) -> (Address, Address) {
    // Register the issuer registry contract
    use issuer_registry::IssuerRegistry;
    let ir_id = env.register(IssuerRegistry, ());
    let ir_client = issuer_registry::IssuerRegistryClient::new(env, &ir_id);

    let admin = Address::generate(env);
    ir_client.initialize(&admin);

    let issuer = Address::generate(env);
    ir_client.add_issuer(
        &issuer,
        &String::from_str(env, "TestIssuer"),
        &String::from_str(env, "desc"),
    );
    ir_client.register_credential_type(
        &issuer,
        &String::from_str(env, "jobs_completed"),
        &String::from_str(env, "Jobs"),
        &String::from_str(env, "desc"),
        &String::from_str(env, "{}"),
        &false,
    );

    (ir_id, issuer)
}

fn setup() -> (Env, ReputationRegistryClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let (ir_id, issuer) = setup_issuer_registry(&env);

    let contract_id = env.register(ReputationRegistry, ());
    let client = ReputationRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &ir_id);

    (env, client, admin, issuer, ir_id)
}

fn make_hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

#[test]
fn test_initialize() {
    let (_, client, _, _, _) = setup();
    assert_eq!(client.get_total_proofs(), 0);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_double_initialize() {
    let (env, client, admin, _, ir_id) = setup();
    let _ = env;
    client.initialize(&admin, &ir_id);
}

#[test]
fn test_register_and_get_proof() {
    let (env, client, _, issuer, _) = setup();
    let owner = Address::generate(&env);

    let proof_hash = make_hash(&env, 1);
    let cred_hash = make_hash(&env, 2);

    client.register_proof(
        &owner,
        &issuer,
        &proof_hash,
        &cred_hash,
        &String::from_str(&env, "jobs_completed"),
        &0u64,
        &String::from_str(&env, "ipfs://abc"),
    );

    assert_eq!(client.get_total_proofs(), 1);
    let proof = client.get_proof(&proof_hash);
    assert_eq!(proof.owner, owner);
    assert!(proof.is_active);
}

#[test]
#[should_panic(expected = "proof already registered")]
fn test_duplicate_proof() {
    let (env, client, _, issuer, _) = setup();
    let owner = Address::generate(&env);
    let proof_hash = make_hash(&env, 1);
    let cred_hash = make_hash(&env, 2);

    client.register_proof(
        &owner, &issuer, &proof_hash, &cred_hash,
        &String::from_str(&env, "jobs_completed"), &0u64, &String::from_str(&env, ""),
    );
    client.register_proof(
        &owner, &issuer, &proof_hash, &cred_hash,
        &String::from_str(&env, "jobs_completed"), &0u64, &String::from_str(&env, ""),
    );
}

#[test]
fn test_revoke_proof() {
    let (env, client, _, issuer, _) = setup();
    let owner = Address::generate(&env);
    let proof_hash = make_hash(&env, 3);

    client.register_proof(
        &owner, &issuer, &proof_hash, &make_hash(&env, 4),
        &String::from_str(&env, "jobs_completed"), &0u64, &String::from_str(&env, ""),
    );

    client.revoke_proof(&proof_hash, &owner);
    let proof = client.get_proof(&proof_hash);
    assert!(!proof.is_active);
}

#[test]
fn test_reputation_score_increases_with_proofs() {
    let (env, client, _, issuer, _) = setup();
    let owner = Address::generate(&env);

    let score_before = client.get_reputation_score(&owner).score;
    assert_eq!(score_before, 0);

    client.register_proof(
        &owner, &issuer, &make_hash(&env, 10), &make_hash(&env, 11),
        &String::from_str(&env, "jobs_completed"), &0u64, &String::from_str(&env, ""),
    );

    let score_after = client.get_reputation_score(&owner).score;
    assert!(score_after > 0);
}

#[test]
fn test_verify_score_threshold() {
    let (env, client, _, issuer, _) = setup();
    let owner = Address::generate(&env);

    client.register_proof(
        &owner, &issuer, &make_hash(&env, 20), &make_hash(&env, 21),
        &String::from_str(&env, "jobs_completed"), &0u64, &String::from_str(&env, ""),
    );

    assert!(client.verify_score_threshold(&owner, &1u32));
    assert!(!client.verify_score_threshold(&owner, &1000u32));
}

#[test]
fn test_get_active_proofs_excludes_revoked() {
    let (env, client, _, issuer, _) = setup();
    let owner = Address::generate(&env);

    client.register_proof(
        &owner, &issuer, &make_hash(&env, 30), &make_hash(&env, 31),
        &String::from_str(&env, "jobs_completed"), &0u64, &String::from_str(&env, ""),
    );
    client.revoke_proof(&make_hash(&env, 30), &owner);

    assert_eq!(client.get_active_user_proofs(&owner).len(), 0);
}

#[test]
fn test_badge_creation_and_award() {
    let (env, client, admin, issuer, _) = setup();
    let user = Address::generate(&env);

    // Create badge
    client.create_badge(
        &String::from_str(&env, "top_dev"),
        &String::from_str(&env, "Top Developer"),
        &String::from_str(&env, "desc"),
        &50u32,
        &Vec::new(&env),
    );

    // Register enough proofs to cross threshold
    client.register_proof(
        &user, &issuer, &make_hash(&env, 40), &make_hash(&env, 41),
        &String::from_str(&env, "jobs_completed"), &0u64, &String::from_str(&env, ""),
    );

    let earned = client.check_and_award_badges(&user);
    assert_eq!(earned.len(), 1);
    assert_eq!(client.get_user_badges(&user).len(), 1);
}

#[test]
fn test_request_and_complete_verification() {
    let (env, client, admin, issuer, _) = setup();
    let owner = Address::generate(&env);
    let requester = Address::generate(&env);

    let proof_hash = make_hash(&env, 50);
    client.register_proof(
        &owner, &issuer, &proof_hash, &make_hash(&env, 51),
        &String::from_str(&env, "jobs_completed"), &0u64, &String::from_str(&env, ""),
    );

    let request_id = client.request_verification(&requester, &owner, &proof_hash);
    client.complete_verification(&request_id, &admin, &true);

    let req = client.get_verification_request(&request_id);
    assert!(req.is_verified);
}
