#![cfg(test)]

use super::issuer_registry::*;
use soroban_sdk::{
    testutils::{storage::Persistent as _, Address as _, Events as _, Ledger as _},
    vec, Address, Env, IntoVal, String, Symbol,
};

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
    client.add_issuer(
        &issuer,
        &String::from_str(&env, "X"),
        &String::from_str(&env, "Y"),
    );
    client.add_issuer(
        &issuer,
        &String::from_str(&env, "X"),
        &String::from_str(&env, "Y"),
    );
}

#[test]
fn test_remove_issuer() {
    let (env, client, _) = setup();
    let issuer = Address::generate(&env);
    client.add_issuer(
        &issuer,
        &String::from_str(&env, "X"),
        &String::from_str(&env, "Y"),
    );
    assert_eq!(client.get_issuer_count(), 1);
    client.remove_issuer(&issuer);
    assert_eq!(client.get_issuer_count(), 0);
    assert!(!client.is_issuer(&issuer));
}

#[test]
fn test_update_issuer_status() {
    let (env, client, _) = setup();
    let issuer = Address::generate(&env);
    client.add_issuer(
        &issuer,
        &String::from_str(&env, "X"),
        &String::from_str(&env, "Y"),
    );
    assert!(client.is_issuer(&issuer));

    client.update_issuer_status(&issuer, &false);
    assert!(!client.is_issuer(&issuer));

    client.update_issuer_status(&issuer, &true);
    assert!(client.is_issuer(&issuer));
}

#[test]
fn test_toggle_lifecycle() {
    let (env, client, _) = setup();
    let issuer = Address::generate(&env);
    client.add_issuer(
        &issuer,
        &String::from_str(&env, "X"),
        &String::from_str(&env, "Y"),
    );

    assert!(client.is_issuer(&issuer));
    assert_eq!(client.get_active_issuers().len(), 1);

    client.update_issuer_status(&issuer, &false);
    assert!(!client.is_issuer(&issuer));
    assert_eq!(client.get_active_issuers().len(), 0);

    client.update_issuer_status(&issuer, &true);
    assert!(client.is_issuer(&issuer));
    assert_eq!(client.get_active_issuers().len(), 1);
}

#[test]
fn test_register_and_verify_credential_type() {
    let (env, client, _) = setup();
    let issuer = Address::generate(&env);
    client.add_issuer(
        &issuer,
        &String::from_str(&env, "X"),
        &String::from_str(&env, "Y"),
    );

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
    client.add_issuer(
        &issuer,
        &String::from_str(&env, "X"),
        &String::from_str(&env, "Y"),
    );
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

fn setup_credential_type(
    env: &Env,
    client: &IssuerRegistryClient<'static>,
) -> (Address, Address, String) {
    let issuer = Address::generate(env);
    let user = Address::generate(env);
    let credential_id = String::from_str(env, "jobs_completed");

    client.add_issuer(
        &issuer,
        &String::from_str(env, "X"),
        &String::from_str(env, "Y"),
    );
    client.register_credential_type(
        &issuer,
        &credential_id,
        &String::from_str(env, "Jobs"),
        &String::from_str(env, "desc"),
        &String::from_str(env, "{}"),
        &false,
    );

    (issuer, user, credential_id)
}

#[test]
fn test_issue_credential_expires_at_zero_sets_no_ttl() {
    let (env, client, _) = setup();
    let (issuer, user, credential_id) = setup_credential_type(&env, &client);

    let hash = soroban_sdk::BytesN::from_array(&env, &[1u8; 32]);
    client.issue_credential(&issuer, &user, &credential_id, &hash, &0u32);

    let credential_key = (issuer, user, credential_id);
    let default_ttl = env.ledger().get().min_persistent_entry_ttl - 1;

    // No extend_ttl call was made, so the entry keeps the default TTL granted
    // by `set` rather than any expires_at-derived value.
    env.as_contract(&client.address, || {
        assert_eq!(
            env.storage().persistent().get_ttl(&credential_key),
            default_ttl
        );
    });
}

#[test]
fn test_issue_credential_past_expires_at_does_not_extend_ttl() {
    let (env, client, _) = setup();
    let (issuer, user, credential_id) = setup_credential_type(&env, &client);

    // A timestamp already behind the current ledger. Since it is far below
    // the default TTL granted on `set`, extend_ttl's threshold check means
    // no extension happens.
    let expires_at = env.ledger().sequence().saturating_sub(1);

    let hash = soroban_sdk::BytesN::from_array(&env, &[1u8; 32]);
    client.issue_credential(&issuer, &user, &credential_id, &hash, &expires_at);

    let credential_key = (issuer, user, credential_id);
    let default_ttl = env.ledger().get().min_persistent_entry_ttl - 1;

    env.as_contract(&client.address, || {
        assert_eq!(
            env.storage().persistent().get_ttl(&credential_key),
            default_ttl
        );
    });
}

#[test]
fn test_issue_credential_future_expires_at_extends_ttl() {
    let (env, client, _) = setup();
    let (issuer, user, credential_id) = setup_credential_type(&env, &client);

    // Comfortably beyond the default TTL granted on `set`, so extend_ttl's
    // threshold check triggers and the TTL is bumped to expires_at.
    let expires_at = env.ledger().get().min_persistent_entry_ttl + 100_000;

    let hash = soroban_sdk::BytesN::from_array(&env, &[1u8; 32]);
    client.issue_credential(&issuer, &user, &credential_id, &hash, &expires_at);

    let credential_key = (issuer, user, credential_id);

    env.as_contract(&client.address, || {
        assert_eq!(
            env.storage().persistent().get_ttl(&credential_key),
            expires_at
        );
    });
}

#[test]
fn test_get_all_and_active_issuers() {
    let (env, client, _) = setup();
    let issuer1 = Address::generate(&env);
    let issuer2 = Address::generate(&env);

    client.add_issuer(
        &issuer1,
        &String::from_str(&env, "A"),
        &String::from_str(&env, ""),
    );
    client.add_issuer(
        &issuer2,
        &String::from_str(&env, "B"),
        &String::from_str(&env, ""),
    );
    client.update_issuer_status(&issuer2, &false);

    assert_eq!(client.get_all_issuers().len(), 2);
    assert_eq!(client.get_active_issuers().len(), 1);
}

#[test]
fn test_get_issuer_count() {
    let (env, client, _) = setup();
    let issuer1 = Address::generate(&env);
    let issuer2 = Address::generate(&env);

    assert_eq!(client.get_issuer_count(), 0);

    client.add_issuer(
        &issuer1,
        &String::from_str(&env, "A"),
        &String::from_str(&env, ""),
    );
    assert_eq!(client.get_issuer_count(), 1);

    client.add_issuer(
        &issuer2,
        &String::from_str(&env, "B"),
        &String::from_str(&env, ""),
    );
    assert_eq!(client.get_issuer_count(), 2);

    client.remove_issuer(&issuer1);
    assert_eq!(client.get_issuer_count(), 1);
}

#[test]
fn test_transfer_admin() {
    let (env, client, _) = setup();
    let new_admin = Address::generate(&env);
    client.transfer_admin(&new_admin);
    // New admin can add issuer (old admin would fail after transfer)
    let issuer = Address::generate(&env);
    client.add_issuer(
        &issuer,
        &String::from_str(&env, "X"),
        &String::from_str(&env, "Y"),
    );
}

#[test]
fn test_add_issuer_emits_event() {
    let (env, client, _) = setup();
    let issuer = Address::generate(&env);
    let name = String::from_str(&env, "Upwork");

    client.add_issuer(
        &issuer,
        &name,
        &String::from_str(&env, "Freelance platform"),
    );

    // The sandbox records all events; find the one with topics ("issuer", "add").
    let events = env.events().all();
    let found = events.iter().any(|(contract_id, topics, _data)| {
        let _ = contract_id;
        topics
            == vec![
                &env,
                Symbol::new(&env, "issuer").into_val(&env),
                Symbol::new(&env, "add").into_val(&env),
            ]
    });

    assert!(found, "expected (\"issuer\", \"add\") event to be emitted");
}
