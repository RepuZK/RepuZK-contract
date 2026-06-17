#![cfg(test)]

use super::issuer_registry::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn setup() -> (Env, IssuerRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(IssuerRegistry, ());
    let client = IssuerRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn test_initialize() {
    let (_, client, _) = setup();
    assert_eq!(client.get_issuer_count(), 0);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_double_initialize() {
    let (env, client, admin) = setup();
    let _ = env;
    client.initialize(&admin);
}

#[test]
fn test_add_and_get_issuer() {
    let (env, client, _) = setup();
    let issuer = Address::generate(&env);

    let result = client.add_issuer(
        &issuer,
        &String::from_str(&env, "Upwork"),
        &String::from_str(&env, "Freelance platform"),
    );
    assert!(result);
    assert_eq!(client.get_issuer_count(), 1);
    assert!(client.is_issuer(&issuer));

    let got = client.get_issuer(&issuer);
    assert_eq!(got.address, issuer);
    assert!(got.is_active);
}

#[test]
#[should_panic(expected = "issuer already exists")]
fn test_add_duplicate_issuer() {
    let (env, client, _) = setup();
    let issuer = Address::generate(&env);
    client.add_issuer(&issuer, &String::from_str(&env, "X"), &String::from_str(&env, "Y"));
    client.add_issuer(&issuer, &String::from_str(&env, "X"), &String::from_str(&env, "Y"));
}

#[test]
fn test_remove_issuer() {
    let (env, client, _) = setup();
    let issuer = Address::generate(&env);
    client.add_issuer(&issuer, &String::from_str(&env, "X"), &String::from_str(&env, "Y"));
    assert_eq!(client.get_issuer_count(), 1);
    client.remove_issuer(&issuer);
    assert_eq!(client.get_issuer_count(), 0);
    assert!(!client.is_issuer(&issuer));
}

#[test]
fn test_update_issuer_status() {
    let (env, client, _) = setup();
    let issuer = Address::generate(&env);
    client.add_issuer(&issuer, &String::from_str(&env, "X"), &String::from_str(&env, "Y"));
    assert!(client.is_issuer(&issuer));

    client.update_issuer_status(&issuer, &false);
    assert!(!client.is_issuer(&issuer));

    client.update_issuer_status(&issuer, &true);
    assert!(client.is_issuer(&issuer));
}

#[test]
fn test_register_and_verify_credential_type() {
    let (env, client, _) = setup();
    let issuer = Address::generate(&env);
    client.add_issuer(&issuer, &String::from_str(&env, "X"), &String::from_str(&env, "Y"));

    client.register_credential_type(
        &issuer,
        &String::from_str(&env, "jobs_completed"),
        &String::from_str(&env, "Jobs Completed"),
        &String::from_str(&env, "Number of jobs completed"),
        &String::from_str(&env, "{}"),
        &true,
    );

    assert!(client.verify_credential_type(&issuer, &String::from_str(&env, "jobs_completed")));
    assert!(!client.verify_credential_type(&issuer, &String::from_str(&env, "unknown")));
}

#[test]
fn test_issue_credential() {
    let (env, client, _) = setup();
    let issuer = Address::generate(&env);
    let user = Address::generate(&env);
    client.add_issuer(&issuer, &String::from_str(&env, "X"), &String::from_str(&env, "Y"));
    client.register_credential_type(
        &issuer,
        &String::from_str(&env, "jobs_completed"),
        &String::from_str(&env, "Jobs"),
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "{}"),
        &false,
    );

    let hash = soroban_sdk::BytesN::from_array(&env, &[1u8; 32]);
    let result = client.issue_credential(
        &issuer,
        &user,
        &String::from_str(&env, "jobs_completed"),
        &hash,
        &0u32,
    );
    assert!(result);
}

#[test]
fn test_get_all_and_active_issuers() {
    let (env, client, _) = setup();
    let issuer1 = Address::generate(&env);
    let issuer2 = Address::generate(&env);

    client.add_issuer(&issuer1, &String::from_str(&env, "A"), &String::from_str(&env, ""));
    client.add_issuer(&issuer2, &String::from_str(&env, "B"), &String::from_str(&env, ""));
    client.update_issuer_status(&issuer2, &false);

    assert_eq!(client.get_all_issuers().len(), 2);
    assert_eq!(client.get_active_issuers().len(), 1);
}

#[test]
fn test_transfer_admin() {
    let (env, client, _) = setup();
    let new_admin = Address::generate(&env);
    client.transfer_admin(&new_admin);
    // New admin can add issuer (old admin would fail after transfer)
    let issuer = Address::generate(&env);
    client.add_issuer(&issuer, &String::from_str(&env, "X"), &String::from_str(&env, "Y"));
}
