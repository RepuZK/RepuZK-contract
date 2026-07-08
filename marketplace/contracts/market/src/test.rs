#![cfg(test)]

use super::marketplace::*;
use soroban_sdk::{
    testutils::{Address as _, Events as _},
    token::StellarAssetClient,
    vec, Address, BytesN, Env, IntoVal, String, Symbol, Vec,
};

/// Set up the full environment: IssuerRegistry + ReputationRegistry + Marketplace + token
struct TestEnv {
    env: Env,
    market_client: ReputationMarketplaceClient<'static>,
    token_address: Address,
    admin: Address,
    issuer: Address,
    provider: Address,
    buyer: Address,
}

impl TestEnv {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // --- IssuerRegistry ---
        let ir_id = env.register(issuer_registry::issuer_registry::IssuerRegistry, ());
        let ir_client = issuer_registry::issuer_registry::IssuerRegistryClient::new(&env, &ir_id);
        let ir_admin = Address::generate(&env);
        ir_client.initialize(&ir_admin);

        let issuer = Address::generate(&env);
        ir_client.add_issuer(&issuer, &String::from_str(&env, "TestIssuer"), &String::from_str(&env, ""));
        ir_client.register_credential_type(
            &issuer,
            &String::from_str(&env, "jobs_completed"),
            &String::from_str(&env, "Jobs"),
            &String::from_str(&env, ""),
            &String::from_str(&env, "{}"),
            &false,
        );

        // --- ReputationRegistry ---
        let rr_id = env.register(reputation_registry::reputation_registry::ReputationRegistry, ());
        let rr_client = reputation_registry::reputation_registry::ReputationRegistryClient::new(&env, &rr_id);
        let rr_admin = Address::generate(&env);
        rr_client.initialize(&rr_admin, &ir_id);

        // --- Token (SAC) ---
        let token_admin = Address::generate(&env);
        let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = sac.address();
        let sac_client = StellarAssetClient::new(&env, &token_address);

        // --- Marketplace ---
        let market_id = env.register(ReputationMarketplace, ());
        let market_client = ReputationMarketplaceClient::new(&env, &market_id);
        let admin = Address::generate(&env);
        let fee_recipient = Address::generate(&env);

        market_client.initialize(&admin, &rr_id, &ir_id, &250u32, &fee_recipient);

        // Register a proof for provider so they can list
        let provider = Address::generate(&env);
        let proof_hash = BytesN::from_array(&env, &[1u8; 32]);
        rr_client.register_proof(
            &provider,
            &issuer,
            &proof_hash,
            &BytesN::from_array(&env, &[2u8; 32]),
            &String::from_str(&env, "jobs_completed"),
            &0u64,
            &String::from_str(&env, ""),
        );

        // Register a proof for buyer
        let buyer = Address::generate(&env);
        let buyer_proof_hash = BytesN::from_array(&env, &[3u8; 32]);
        rr_client.register_proof(
            &buyer,
            &issuer,
            &buyer_proof_hash,
            &BytesN::from_array(&env, &[4u8; 32]),
            &String::from_str(&env, "jobs_completed"),
            &0u64,
            &String::from_str(&env, ""),
        );

        // Mint tokens to buyer
        sac_client.mint(&buyer, &10_000i128);

        TestEnv {
            env,
            market_client,
            token_address,
            admin,
            issuer,
            provider,
            buyer,
        }
    }

    fn create_listing(&self, price: i128) -> u64 {
        self.market_client.create_listing(
            &self.provider,
            &String::from_str(&self.env, "Test Service"),
            &String::from_str(&self.env, "Description"),
            &String::from_str(&self.env, "development"),
            &price,
            &self.token_address,
            &0u32,
            &Vec::new(&self.env),
            &7u32,
        )
    }
}

#[test]
fn test_create_listing() {
    let t = TestEnv::new();
    let listing_id = t.create_listing(1000);
    assert_eq!(listing_id, 1);

    let listing = t.market_client.get_listing(&listing_id);
    assert_eq!(listing.price, 1000);
    assert!(listing.is_active);
    assert_eq!(t.market_client.get_active_listings().len(), 1);
}

#[test]
#[should_panic(expected = "price below minimum")]
fn test_listing_price_too_low() {
    let t = TestEnv::new();
    t.create_listing(10);
}

#[test]
fn test_purchase_service_escrows_tokens() {
    let t = TestEnv::new();
    let listing_id = t.create_listing(1000);

    let token_client = soroban_sdk::token::Client::new(&t.env, &t.token_address);
    let buyer_balance_before = token_client.balance(&t.buyer);

    let order_id = t.market_client.purchase_service(
        &t.buyer,
        &listing_id,
        &BytesN::from_array(&t.env, &[9u8; 32]),
    );

    assert_eq!(order_id, 1);
    let buyer_balance_after = token_client.balance(&t.buyer);
    assert_eq!(buyer_balance_before - buyer_balance_after, 1000);

    let order = t.market_client.get_order(&order_id);
    assert_eq!(order.status, OrderStatus::Paid);
}

#[test]
fn test_order_lifecycle_and_seller_payout() {
    let t = TestEnv::new();
    let listing_id = t.create_listing(1000);
    let order_id = t.market_client.purchase_service(
        &t.buyer,
        &listing_id,
        &BytesN::from_array(&t.env, &[9u8; 32]),
    );

    t.market_client.start_order(&t.provider, &order_id);
    assert_eq!(t.market_client.get_order(&order_id).status, OrderStatus::InProgress);

    let token_client = soroban_sdk::token::Client::new(&t.env, &t.token_address);
    let seller_balance_before = token_client.balance(&t.provider);

    t.market_client.complete_order(
        &t.provider,
        &order_id,
        &BytesN::from_array(&t.env, &[0u8; 32]),
    );

    let order = t.market_client.get_order(&order_id);
    assert_eq!(order.status, OrderStatus::Completed);

    // Seller receives 97.5% (250 bps fee)
    let seller_balance_after = token_client.balance(&t.provider);
    assert_eq!(seller_balance_after - seller_balance_before, 975);
}

#[test]
fn test_complete_order_emits_event() {
    let t = TestEnv::new();
    let listing_id = t.create_listing(1000);
    let order_id = t.market_client.purchase_service(
        &t.buyer,
        &listing_id,
        &BytesN::from_array(&t.env, &[9u8; 32]),
    );

    t.market_client.start_order(&t.provider, &order_id);
    let completed_at = t.env.ledger().timestamp();
    t.market_client.complete_order(
        &t.provider,
        &order_id,
        &BytesN::from_array(&t.env, &[0u8; 32]),
    );

    // events().all() only returns events from the most recent contract
    // invocation, so this must run right after complete_order and before
    // any other client call. Filter to the marketplace contract since
    // release_to_seller's token transfers also emit events here.
    assert_eq!(
        t.env.events().all().filter_by_contract(&t.market_client.address),
        vec![
            &t.env,
            (
                t.market_client.address.clone(),
                (Symbol::new(&t.env, "order"), Symbol::new(&t.env, "complete")).into_val(&t.env),
                (order_id, t.provider.clone(), t.buyer.clone(), 1000i128, completed_at)
                    .into_val(&t.env),
            ),
        ]
    );
}

#[test]
fn test_dispute_and_resolve_refund() {
    let t = TestEnv::new();
    let listing_id = t.create_listing(1000);
    let order_id = t.market_client.purchase_service(
        &t.buyer,
        &listing_id,
        &BytesN::from_array(&t.env, &[9u8; 32]),
    );

    t.market_client.start_order(&t.provider, &order_id);
    t.market_client.raise_dispute(
        &t.buyer,
        &order_id,
        &String::from_str(&t.env, "work not delivered"),
    );
    assert_eq!(t.market_client.get_order(&order_id).status, OrderStatus::Disputed);

    let token_client = soroban_sdk::token::Client::new(&t.env, &t.token_address);
    let buyer_before = token_client.balance(&t.buyer);

    // Admin refunds buyer
    t.market_client.resolve_dispute(&t.admin, &order_id, &false);

    let buyer_after = token_client.balance(&t.buyer);
    assert_eq!(buyer_after - buyer_before, 1000);
    assert_eq!(t.market_client.get_order(&order_id).status, OrderStatus::Refunded);
}

#[test]
fn test_dispute_resolve_in_seller_favor() {
    let t = TestEnv::new();
    let listing_id = t.create_listing(1000);
    let order_id = t.market_client.purchase_service(
        &t.buyer,
        &listing_id,
        &BytesN::from_array(&t.env, &[9u8; 32]),
    );

    t.market_client.start_order(&t.provider, &order_id);
    t.market_client.raise_dispute(
        &t.buyer,
        &order_id,
        &String::from_str(&t.env, "dispute"),
    );

    let token_client = soroban_sdk::token::Client::new(&t.env, &t.token_address);
    let seller_before = token_client.balance(&t.provider);

    t.market_client.resolve_dispute(&t.admin, &order_id, &true);

    let seller_after = token_client.balance(&t.provider);
    assert_eq!(seller_after - seller_before, 975); // minus 2.5% fee
    assert_eq!(t.market_client.get_order(&order_id).status, OrderStatus::Completed);
}

#[test]
fn test_leave_feedback() {
    let t = TestEnv::new();
    let listing_id = t.create_listing(1000);
    let order_id = t.market_client.purchase_service(
        &t.buyer,
        &listing_id,
        &BytesN::from_array(&t.env, &[9u8; 32]),
    );

    t.market_client.start_order(&t.provider, &order_id);
    t.market_client.complete_order(
        &t.provider,
        &order_id,
        &BytesN::from_array(&t.env, &[0u8; 32]),
    );

    let feedback_id = t.market_client.leave_feedback(
        &t.buyer,
        &order_id,
        &5u32,
        &String::from_str(&t.env, "Excellent work!"),
        &BytesN::from_array(&t.env, &[1u8; 32]),
    );

    assert_eq!(feedback_id, 1);
    let fb = t.market_client.get_feedback(&feedback_id);
    assert_eq!(fb.rating, 5);
    assert!(fb.is_verified);

    let (avg, count) = t.market_client.get_user_rating(&t.provider);
    assert_eq!(avg, 5);
    assert_eq!(count, 1);
}

#[test]
fn test_update_listing() {
    let t = TestEnv::new();
    let listing_id = t.create_listing(1000);
    t.market_client.update_listing(&t.provider, &listing_id, &Some(2000i128), &None);
    assert_eq!(t.market_client.get_listing(&listing_id).price, 2000);

    t.market_client.update_listing(&t.provider, &listing_id, &None, &Some(false));
    assert!(!t.market_client.get_listing(&listing_id).is_active);
}

#[test]
fn test_get_listings_by_category() {
    let t = TestEnv::new();
    t.create_listing(1000);
    let listings = t.market_client.get_listings_by_category(&String::from_str(&t.env, "development"));
    assert_eq!(listings.len(), 1);

    let empty = t.market_client.get_listings_by_category(&String::from_str(&t.env, "other"));
    assert_eq!(empty.len(), 0);
}

#[test]
fn test_get_provider_stats() {
    let t = TestEnv::new();

    t.create_listing(1000);
    t.create_listing(2000);

    let order1 = t.market_client.purchase_service(
        &t.buyer,
        &1,
        &BytesN::from_array(&t.env, &[9u8; 32]),
    );

    let order2 = t.market_client.purchase_service(
        &t.buyer,
        &2,
        &BytesN::from_array(&t.env, &[9u8; 32]),
    );

    assert_eq!(t.market_client.get_provider_stats(&t.provider).total_listings, 2);
    assert_eq!(t.market_client.get_provider_stats(&t.provider).total_orders, 2);
    assert_eq!(t.market_client.get_provider_stats(&t.provider).completed_orders, 0);
    assert_eq!(t.market_client.get_provider_stats(&t.provider).disputed_orders, 0);
    assert_eq!(t.market_client.get_provider_stats(&t.provider).total_revenue, 0);

    t.market_client.start_order(&t.provider, &order1);
    t.market_client.complete_order(
        &t.provider,
        &order1,
        &BytesN::from_array(&t.env, &[0u8; 32]),
    );
    t.market_client.leave_feedback(
        &t.buyer,
        &order1,
        &5u32,
        &String::from_str(&t.env, "Great!"),
        &BytesN::from_array(&t.env, &[1u8; 32]),
    );

    t.market_client.start_order(&t.provider, &order2);
    t.market_client.raise_dispute(
        &t.buyer,
        &order2,
        &String::from_str(&t.env, "dispute"),
    );

    let stats = t.market_client.get_provider_stats(&t.provider);
    assert_eq!(stats.total_listings, 2);
    assert_eq!(stats.total_orders, 2);
    assert_eq!(stats.completed_orders, 1);
    assert_eq!(stats.disputed_orders, 1);
    assert_eq!(stats.total_revenue, 975);
    assert_eq!(stats.avg_rating, 5);
}
