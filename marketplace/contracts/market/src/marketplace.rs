#![no_std]
use soroban_sdk::{
    contract, contractimpl, contractclient, contracttype, token, Address, Env, String, Vec,
    BytesN, Symbol,
};

// ==================== Cross-Contract Interfaces ====================

#[contractclient(name = "ReputationRegistryClient")]
pub trait ReputationRegistryInterface {
    fn get_score_value(env: Env, user: Address) -> u32;
    fn has_credential(env: Env, user: Address, credential_type: String) -> bool;
}

// ==================== Data Structures ====================

#[contracttype]
#[derive(Clone)]
pub struct Listing {
    pub listing_id: u64,
    pub provider: Address,
    pub title: String,
    pub description: String,
    pub category: String,
    pub price: i128,
    pub token_address: Address,
    pub min_reputation_score: u32,
    pub required_credentials: Vec<String>,
    pub delivery_days: u32,
    pub is_active: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct Order {
    pub order_id: u64,
    pub listing_id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub amount: i128,
    pub token_address: Address,
    pub status: OrderStatus,
    pub payment_tx_hash: BytesN<32>,
    pub created_at: u64,
    pub paid_at: u64,
    pub completed_at: u64,
    pub delivery_deadline: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct Feedback {
    pub feedback_id: u64,
    pub order_id: u64,
    pub reviewer: Address,
    pub reviewee: Address,
    pub rating: u32,
    pub comment: String,
    pub completion_proof: BytesN<32>,
    pub created_at: u64,
    pub is_verified: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct ReputationVerification {
    pub user: Address,
    pub score: u32,
    pub credentials: Vec<String>,
    pub verified_at: u64,
    pub proof_hash: BytesN<32>,
    pub is_valid: bool,
}

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum OrderStatus {
    Created,
    Paid,
    InProgress,
    Completed,
    Disputed,
    Cancelled,
    Refunded,
}

// ==================== Storage Keys ====================

#[contracttype]
pub enum DataKey {
    Listing(u64),
    AllListings,
    ProviderListings(Address),
    CategoryListings(String),
    Order(u64),
    BuyerOrders(Address),
    SellerOrders(Address),
    Feedback(u64),
    OrderFeedback(u64),
    UserFeedbackReceived(Address),
    UserFeedbackGiven(Address),
    ReputationVerification(Address),
    NextListingId,
    NextOrderId,
    NextFeedbackId,
    PlatformFeeBps,
    FeeRecipient,
    MinListingPrice,
    EscrowDurationDays,
    Admin,
    ReputationRegistry,
    IssuerRegistry,
}

// ==================== Main Contract ====================

#[contract]
pub struct ReputationMarketplace;

#[contractimpl]
impl ReputationMarketplace {
    // ============ Initialization ============

    pub fn initialize(
        env: Env,
        admin: Address,
        reputation_registry: Address,
        issuer_registry: Address,
        platform_fee_bps: u32,
        fee_recipient: Address,
    ) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::ReputationRegistry, &reputation_registry);
        env.storage().instance().set(&DataKey::IssuerRegistry, &issuer_registry);
        env.storage().instance().set(&DataKey::PlatformFeeBps, &platform_fee_bps);
        env.storage().instance().set(&DataKey::FeeRecipient, &fee_recipient);
        env.storage().instance().set(&DataKey::MinListingPrice, &100i128);
        env.storage().instance().set(&DataKey::EscrowDurationDays, &14u32);

        env.storage().instance().set(&DataKey::NextListingId, &1u64);
        env.storage().instance().set(&DataKey::NextOrderId, &1u64);
        env.storage().instance().set(&DataKey::NextFeedbackId, &1u64);
    }

    // ============ Listing Management ============

    pub fn create_listing(
        env: Env,
        provider: Address,
        title: String,
        description: String,
        category: String,
        price: i128,
        token_address: Address,
        min_reputation_score: u32,
        required_credentials: Vec<String>,
        delivery_days: u32,
    ) -> u64 {
        provider.require_auth();

        let min_price: i128 = env.storage().instance().get(&DataKey::MinListingPrice).unwrap_or(100);
        if price < min_price {
            panic!("price below minimum");
        }

        if delivery_days == 0 || delivery_days > 90 {
            panic!("invalid delivery days");
        }

        let listing_id: u64 = env.storage().instance().get(&DataKey::NextListingId).unwrap_or(1);
        let now = env.ledger().timestamp();

        let listing = Listing {
            listing_id,
            provider: provider.clone(),
            title: title.clone(),
            description,
            category: category.clone(),
            price,
            token_address,
            min_reputation_score,
            required_credentials,
            delivery_days,
            is_active: true,
            created_at: now,
            updated_at: now,
        };

        env.storage().instance().set(&DataKey::Listing(listing_id), &listing);

        let mut all_listings: Vec<u64> = env
            .storage().instance().get(&DataKey::AllListings).unwrap_or(Vec::new(&env));
        all_listings.push_back(listing_id);
        env.storage().instance().set(&DataKey::AllListings, &all_listings);

        let mut provider_listings: Vec<u64> = env
            .storage().instance().get(&DataKey::ProviderListings(provider.clone())).unwrap_or(Vec::new(&env));
        provider_listings.push_back(listing_id);
        env.storage().instance().set(&DataKey::ProviderListings(provider.clone()), &provider_listings);

        let mut category_listings: Vec<u64> = env
            .storage().instance().get(&DataKey::CategoryListings(category.clone())).unwrap_or(Vec::new(&env));
        category_listings.push_back(listing_id);
        env.storage().instance().set(&DataKey::CategoryListings(category), &category_listings);

        env.storage().instance().set(&DataKey::NextListingId, &(listing_id + 1));

        let topics = (Symbol::new(&env, "listing"), Symbol::new(&env, "create"));
        env.events().publish(topics, (listing_id, provider, title, price));

        listing_id
    }

    // ============ Reputation Verification ============

    fn get_user_reputation_score(env: &Env, user: &Address) -> u32 {
        let registry: Address = env.storage().instance().get(&DataKey::ReputationRegistry).unwrap();
        let client = ReputationRegistryClient::new(env, &registry);
        client.get_score_value(user)
    }

    fn has_user_credential(env: &Env, user: &Address, credential: &String) -> bool {
        let registry: Address = env.storage().instance().get(&DataKey::ReputationRegistry).unwrap();
        let client = ReputationRegistryClient::new(env, &registry);
        client.has_credential(user, credential)
    }

    pub fn verify_reputation(
        env: Env,
        user: Address,
        required_score: u32,
        required_credentials: Vec<String>,
        zk_proof_hash: BytesN<32>,
    ) -> bool {
        user.require_auth();

        let user_score = Self::get_user_reputation_score(&env, &user);
        if user_score < required_score {
            panic!("reputation score too low");
        }

        for i in 0..required_credentials.len() {
            let cred = required_credentials.get(i).unwrap();
            if !Self::has_user_credential(&env, &user, &cred) {
                panic!("missing required credential");
            }
        }

        let verification = ReputationVerification {
            user: user.clone(),
            score: user_score,
            credentials: required_credentials,
            verified_at: env.ledger().timestamp(),
            proof_hash: zk_proof_hash,
            is_valid: true,
        };
        env.storage().instance().set(&DataKey::ReputationVerification(user), &verification);

        true
    }

    // ============ Order Management ============

    pub fn purchase_service(
        env: Env,
        buyer: Address,
        listing_id: u64,
        zk_proof_hash: BytesN<32>,
    ) -> u64 {
        buyer.require_auth();

        let listing: Listing = env
            .storage().instance().get(&DataKey::Listing(listing_id)).expect("listing not found");

        if !listing.is_active {
            panic!("listing is not active");
        }

        if listing.provider == buyer {
            panic!("cannot purchase own listing");
        }

        // Verify buyer's reputation meets listing requirements
        let buyer_score = Self::get_user_reputation_score(&env, &buyer);
        if buyer_score < listing.min_reputation_score {
            panic!("reputation score too low");
        }
        for i in 0..listing.required_credentials.len() {
            let cred = listing.required_credentials.get(i).unwrap();
            if !Self::has_user_credential(&env, &buyer, &cred) {
                panic!("missing required credential");
            }
        }

        let order_id: u64 = env.storage().instance().get(&DataKey::NextOrderId).unwrap_or(1);
        let now = env.ledger().timestamp();
        let deadline = now + (listing.delivery_days as u64 * 86400);

        // Escrow: transfer tokens from buyer to this contract
        let token_client = token::Client::new(&env, &listing.token_address);
        token_client.transfer(&buyer, &env.current_contract_address(), &listing.price);

        let order = Order {
            order_id,
            listing_id,
            buyer: buyer.clone(),
            seller: listing.provider.clone(),
            amount: listing.price,
            token_address: listing.token_address.clone(),
            status: OrderStatus::Paid,
            payment_tx_hash: zk_proof_hash,
            created_at: now,
            paid_at: now,
            completed_at: 0,
            delivery_deadline: deadline,
        };

        env.storage().instance().set(&DataKey::Order(order_id), &order);

        let mut buyer_orders: Vec<u64> = env
            .storage().instance().get(&DataKey::BuyerOrders(buyer.clone())).unwrap_or(Vec::new(&env));
        buyer_orders.push_back(order_id);
        env.storage().instance().set(&DataKey::BuyerOrders(buyer.clone()), &buyer_orders);

        let mut seller_orders: Vec<u64> = env
            .storage().instance().get(&DataKey::SellerOrders(listing.provider.clone())).unwrap_or(Vec::new(&env));
        seller_orders.push_back(order_id);
        env.storage().instance().set(&DataKey::SellerOrders(listing.provider.clone()), &seller_orders);

        env.storage().instance().set(&DataKey::NextOrderId, &(order_id + 1));

        let topics = (Symbol::new(&env, "order"), Symbol::new(&env, "create"));
        env.events().publish(topics, (order_id, listing_id, buyer, listing.provider, listing.price));

        order_id
    }

    pub fn start_order(env: Env, seller: Address, order_id: u64) -> bool {
        seller.require_auth();

        let mut order: Order = env
            .storage().instance().get(&DataKey::Order(order_id)).expect("order not found");

        if order.seller != seller {
            panic!("not order seller");
        }
        if order.status != OrderStatus::Paid {
            panic!("order cannot be started");
        }

        order.status = OrderStatus::InProgress;
        env.storage().instance().set(&DataKey::Order(order_id), &order);

        true
    }

    pub fn complete_order(env: Env, seller: Address, order_id: u64, _completion_proof: BytesN<32>) -> bool {
        seller.require_auth();

        let mut order: Order = env
            .storage().instance().get(&DataKey::Order(order_id)).expect("order not found");

        if order.seller != seller {
            panic!("not order seller");
        }
        if order.status != OrderStatus::InProgress {
            panic!("order not in progress");
        }

        // Release escrowed funds: pay seller minus platform fee
        Self::release_to_seller(&env, &order);

        order.status = OrderStatus::Completed;
        order.completed_at = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::Order(order_id), &order);

        true
    }

    /// Release escrowed amount to seller minus platform fee; fee goes to fee_recipient
    fn release_to_seller(env: &Env, order: &Order) {
        let fee_bps: u32 = env.storage().instance().get(&DataKey::PlatformFeeBps).unwrap_or(250);
        let fee_recipient: Address = env.storage().instance().get(&DataKey::FeeRecipient).unwrap();

        let fee = (order.amount * fee_bps as i128) / 10_000;
        let seller_amount = order.amount - fee;

        let token_client = token::Client::new(env, &order.token_address);
        token_client.transfer(&env.current_contract_address(), &order.seller, &seller_amount);
        if fee > 0 {
            token_client.transfer(&env.current_contract_address(), &fee_recipient, &fee);
        }
    }

    // ============ Dispute Resolution ============

    pub fn raise_dispute(env: Env, buyer: Address, order_id: u64, _reason: String) -> bool {
        buyer.require_auth();

        let mut order: Order = env
            .storage().instance().get(&DataKey::Order(order_id)).expect("order not found");

        if order.buyer != buyer {
            panic!("only buyer can raise dispute");
        }
        if order.status != OrderStatus::InProgress && order.status != OrderStatus::Paid {
            panic!("cannot dispute order in current status");
        }

        order.status = OrderStatus::Disputed;
        env.storage().instance().set(&DataKey::Order(order_id), &order);

        let topics = (Symbol::new(&env, "dispute"), Symbol::new(&env, "raise"));
        env.events().publish(topics, (order_id, buyer, _reason));

        true
    }

    /// Admin resolves a dispute: release_to_seller=true pays seller, false refunds buyer
    pub fn resolve_dispute(env: Env, admin: Address, order_id: u64, release_to_seller: bool) -> bool {
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("not admin");
        }
        admin.require_auth();

        let mut order: Order = env
            .storage().instance().get(&DataKey::Order(order_id)).expect("order not found");

        if order.status != OrderStatus::Disputed {
            panic!("order not in dispute");
        }

        let token_client = token::Client::new(&env, &order.token_address);

        if release_to_seller {
            Self::release_to_seller(&env, &order);
            order.status = OrderStatus::Completed;
        } else {
            // Refund buyer in full
            token_client.transfer(&env.current_contract_address(), &order.buyer, &order.amount);
            order.status = OrderStatus::Refunded;
        }

        order.completed_at = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::Order(order_id), &order);

        let topics = (Symbol::new(&env, "dispute"), Symbol::new(&env, "resolve"));
        env.events().publish(topics, (order_id, release_to_seller));

        true
    }

    // ============ Feedback System ============

    pub fn leave_feedback(
        env: Env,
        reviewer: Address,
        order_id: u64,
        rating: u32,
        comment: String,
        completion_proof: BytesN<32>,
    ) -> u64 {
        reviewer.require_auth();

        if rating < 1 || rating > 5 {
            panic!("rating must be between 1 and 5");
        }

        let order: Order = env
            .storage().instance().get(&DataKey::Order(order_id)).expect("order not found");

        if order.buyer != reviewer {
            panic!("only buyer can leave feedback");
        }
        if order.status != OrderStatus::Completed {
            panic!("order not completed yet");
        }
        if env.storage().instance().has(&DataKey::OrderFeedback(order_id)) {
            panic!("feedback already submitted");
        }

        let is_verified = {
            let empty = BytesN::from_array(&env, &[0u8; 32]);
            completion_proof != empty
        };

        let feedback_id: u64 = env.storage().instance().get(&DataKey::NextFeedbackId).unwrap_or(1);
        let now = env.ledger().timestamp();

        let feedback = Feedback {
            feedback_id,
            order_id,
            reviewer: reviewer.clone(),
            reviewee: order.seller.clone(),
            rating,
            comment,
            completion_proof,
            created_at: now,
            is_verified,
        };

        env.storage().instance().set(&DataKey::Feedback(feedback_id), &feedback);
        env.storage().instance().set(&DataKey::OrderFeedback(order_id), &feedback_id);

        let mut received_feedback: Vec<u64> = env
            .storage().instance().get(&DataKey::UserFeedbackReceived(order.seller.clone())).unwrap_or(Vec::new(&env));
        received_feedback.push_back(feedback_id);
        env.storage().instance().set(&DataKey::UserFeedbackReceived(order.seller.clone()), &received_feedback);

        let mut given_feedback: Vec<u64> = env
            .storage().instance().get(&DataKey::UserFeedbackGiven(reviewer.clone())).unwrap_or(Vec::new(&env));
        given_feedback.push_back(feedback_id);
        env.storage().instance().set(&DataKey::UserFeedbackGiven(reviewer.clone()), &given_feedback);

        env.storage().instance().set(&DataKey::NextFeedbackId, &(feedback_id + 1));

        let topics = (Symbol::new(&env, "feedback"), Symbol::new(&env, "submit"));
        env.events().publish(topics, (feedback_id, order_id, reviewer, order.seller, rating));

        feedback_id
    }

    // ============ Query Functions ============

    pub fn get_listing(env: Env, listing_id: u64) -> Listing {
        env.storage().instance().get(&DataKey::Listing(listing_id)).expect("listing not found")
    }

    pub fn get_active_listings(env: Env) -> Vec<Listing> {
        let all_listing_ids: Vec<u64> = env
            .storage().instance().get(&DataKey::AllListings).unwrap_or(Vec::new(&env));

        let mut active_listings = Vec::new(&env);
        for i in 0..all_listing_ids.len() {
            let id = all_listing_ids.get(i).unwrap();
            if let Some(listing) = env.storage().instance().get::<DataKey, Listing>(&DataKey::Listing(id)) {
                if listing.is_active {
                    active_listings.push_back(listing);
                }
            }
        }
        active_listings
    }

    pub fn get_order(env: Env, order_id: u64) -> Order {
        env.storage().instance().get(&DataKey::Order(order_id)).expect("order not found")
    }

    pub fn get_buyer_orders(env: Env, buyer: Address) -> Vec<Order> {
        let order_ids: Vec<u64> = env
            .storage().instance().get(&DataKey::BuyerOrders(buyer)).unwrap_or(Vec::new(&env));

        let mut orders = Vec::new(&env);
        for i in 0..order_ids.len() {
            let id = order_ids.get(i).unwrap();
            if let Some(order) = env.storage().instance().get(&DataKey::Order(id)) {
                orders.push_back(order);
            }
        }
        orders
    }

    pub fn get_seller_orders(env: Env, seller: Address) -> Vec<Order> {
        let order_ids: Vec<u64> = env
            .storage().instance().get(&DataKey::SellerOrders(seller)).unwrap_or(Vec::new(&env));

        let mut orders = Vec::new(&env);
        for i in 0..order_ids.len() {
            let id = order_ids.get(i).unwrap();
            if let Some(order) = env.storage().instance().get(&DataKey::Order(id)) {
                orders.push_back(order);
            }
        }
        orders
    }

    pub fn get_feedback(env: Env, feedback_id: u64) -> Feedback {
        env.storage().instance().get(&DataKey::Feedback(feedback_id)).expect("feedback not found")
    }

    pub fn get_user_feedback_received(env: Env, user: Address) -> Vec<Feedback> {
        Self::get_user_feedback_received_internal(&env, user)
    }

    fn get_user_feedback_received_internal(env: &Env, user: Address) -> Vec<Feedback> {
        let feedback_ids: Vec<u64> = env
            .storage().instance().get(&DataKey::UserFeedbackReceived(user)).unwrap_or(Vec::new(env));

        let mut feedbacks = Vec::new(env);
        for i in 0..feedback_ids.len() {
            let id = feedback_ids.get(i).unwrap();
            if let Some(fb) = env.storage().instance().get(&DataKey::Feedback(id)) {
                feedbacks.push_back(fb);
            }
        }
        feedbacks
    }

    pub fn get_user_rating(env: Env, user: Address) -> (u32, u32) {
        let feedbacks = Self::get_user_feedback_received(env, user);
        let mut total_rating = 0u32;
        for i in 0..feedbacks.len() {
            total_rating += feedbacks.get(i).unwrap().rating;
        }
        let count = feedbacks.len() as u32;
        let average = if count > 0 { total_rating / count } else { 0 };
        (average, count)
    }

    pub fn get_listings_by_category(env: Env, category: String) -> Vec<Listing> {
        let listing_ids: Vec<u64> = env
            .storage().instance().get(&DataKey::CategoryListings(category)).unwrap_or(Vec::new(&env));

        let mut listings = Vec::new(&env);
        for i in 0..listing_ids.len() {
            let id = listing_ids.get(i).unwrap();
            if let Some(listing) = env.storage().instance().get::<DataKey, Listing>(&DataKey::Listing(id)) {
                if listing.is_active {
                    listings.push_back(listing);
                }
            }
        }
        listings
    }

    pub fn update_listing(
        env: Env,
        provider: Address,
        listing_id: u64,
        new_price: Option<i128>,
        new_is_active: Option<bool>,
    ) -> bool {
        provider.require_auth();

        let mut listing: Listing = env
            .storage().instance().get(&DataKey::Listing(listing_id)).expect("listing not found");

        if listing.provider != provider {
            panic!("not listing owner");
        }

        if let Some(price) = new_price {
            let min_price: i128 = env.storage().instance().get(&DataKey::MinListingPrice).unwrap_or(100);
            if price >= min_price {
                listing.price = price;
            }
        }
        if let Some(is_active) = new_is_active {
            listing.is_active = is_active;
        }

        listing.updated_at = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::Listing(listing_id), &listing);

        true
    }

    pub fn get_platform_stats(env: Env) -> (u32, u32, u32) {
        let total_listings = Self::get_active_listings(env.clone()).len() as u32;
        let fee_bps: u32 = env.storage().instance().get(&DataKey::PlatformFeeBps).unwrap_or(250);
        let min_price: i128 = env.storage().instance().get(&DataKey::MinListingPrice).unwrap_or(100);
        (total_listings, fee_bps, min_price as u32)
    }
}
