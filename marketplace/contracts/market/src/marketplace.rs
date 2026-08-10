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
pub struct ProviderStats {
    pub total_listings: u32,
    pub total_orders: u32,
    pub completed_orders: u32,
    pub disputed_orders: u32,
    pub avg_rating: u32,
    pub total_revenue: i128,
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

    /// Initialize the contract: set the admin, linked registries, platform
    /// fee, fee recipient, and default `MinListingPrice` (100) /
    /// `EscrowDurationDays` (14). Also zeroes the listing/order/feedback id
    /// counters (all start at 1).
    ///
    /// Must be called exactly once after deployment.
    ///
    /// # Panics
    /// - `"already initialized"` — the contract has already been
    ///   initialized.
    /// - `"fee_bps must be <= 10000"` — `platform_fee_bps` exceeds 10000
    ///   (100%). Rejecting this at setup time avoids a later panic inside
    ///   `release_to_seller` (called from `complete_order` /
    ///   `resolve_dispute`), where a fee above 100% would make the computed
    ///   `seller_amount` negative and abort every order completion.
    ///
    /// # Auth
    /// Requires authorization from `admin`.
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

        if platform_fee_bps > 10_000 {
            panic!("fee_bps must be <= 10000");
        }

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

    /// Create a new service listing for `provider`.
    ///
    /// Stores the listing, appends its id to the global `AllListings`
    /// index, the provider's `ProviderListings` index, and the
    /// `CategoryListings` index for `category`, then increments
    /// `NextListingId`. Emits a `("listing", "create")` event.
    ///
    /// Returns the newly assigned `listing_id`.
    ///
    /// # Panics
    /// - `"price below minimum"` — `price` is below `MinListingPrice`.
    /// - `"invalid delivery days"` — `delivery_days` is `0` or greater than
    ///   `90`.
    ///
    /// # Auth
    /// Requires authorization from `provider`.
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

    /// Verify that `user` meets `required_score` and holds every credential
    /// in `required_credentials`, via cross-contract calls to the linked
    /// Reputation Registry. Stores a `ReputationVerification` record for
    /// `user` on success.
    ///
    /// Returns `true` on success.
    ///
    /// # Panics
    /// - `"reputation score too low"` — `user`'s score is below
    ///   `required_score`.
    /// - `"missing required credential"` — `user` is missing one of
    ///   `required_credentials`.
    ///
    /// # Auth
    /// Requires authorization from `user`.
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

    /// Purchase the service in `listing_id`: verify the buyer meets the
    /// listing's `min_reputation_score` and `required_credentials`, escrow
    /// `listing.price` from `buyer` into this contract, and create a new
    /// `Order` in `Paid` status with `delivery_deadline = now +
    /// listing.delivery_days * 86400`. Emits an `("order", "create")`
    /// event.
    ///
    /// Returns the newly assigned `order_id`.
    ///
    /// # Panics
    /// - `"listing not found"` — no listing with `listing_id` exists.
    /// - `"listing is not active"` — the listing has been deactivated.
    /// - `"cannot purchase own listing"` — `buyer` is the listing's
    ///   provider.
    /// - `"reputation score too low"` — buyer's score is below
    ///   `listing.min_reputation_score`.
    /// - `"missing required credential"` — buyer is missing one of
    ///   `listing.required_credentials`.
    ///
    /// # Auth
    /// Requires authorization from `buyer`.
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

    /// Transition an order from `Paid` to `InProgress`, signalling that the
    /// seller has begun work.
    ///
    /// Returns `true` on success.
    ///
    /// # Panics
    /// - `"order not found"` — no order with `order_id` exists.
    /// - `"not order seller"` — `seller` is not the order's seller.
    /// - `"order cannot be started"` — the order is not in `Paid` status.
    ///
    /// # Auth
    /// Requires authorization from `seller`.
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

    /// Complete an `InProgress` order: release the escrowed amount to the
    /// seller (minus the platform fee, which goes to `FeeRecipient`), mark
    /// the order `Completed`, and emit an `("order", "complete")` event.
    ///
    /// Returns `true` on success.
    ///
    /// # Panics
    /// - `"order not found"` — no order with `order_id` exists.
    /// - `"not order seller"` — `seller` is not the order's seller.
    /// - `"order not in progress"` — the order is not in `InProgress`
    ///   status.
    ///
    /// # Auth
    /// Requires authorization from `seller`.
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

        let topics = (Symbol::new(&env, "order"), Symbol::new(&env, "complete"));
        env.events().publish(
            topics,
            (order_id, order.seller.clone(), order.buyer.clone(), order.amount, order.completed_at),
        );

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

    /// Raise a dispute on an order that is `Paid` or `InProgress`, once its
    /// `delivery_deadline` has passed. Marks the order `Disputed` and emits
    /// a `("dispute", "raise")` event; an admin then resolves it via
    /// `resolve_dispute`.
    ///
    /// Returns `true` on success.
    ///
    /// # Panics
    /// - `"order not found"` — no order with `order_id` exists.
    /// - `"only buyer can raise dispute"` — `buyer` is not the order's
    ///   buyer.
    /// - `"cannot dispute order in current status"` — the order is not in
    ///   `Paid` or `InProgress` status.
    /// - `"cannot dispute before delivery deadline"` — the current ledger
    ///   timestamp is still before `order.delivery_deadline`.
    ///
    /// # Auth
    /// Requires authorization from `buyer`.
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
        if env.ledger().timestamp() < order.delivery_deadline {
            panic!("cannot dispute before delivery deadline");
        }

        order.status = OrderStatus::Disputed;
        env.storage().instance().set(&DataKey::Order(order_id), &order);

        let topics = (Symbol::new(&env, "dispute"), Symbol::new(&env, "raise"));
        env.events().publish(topics, (order_id, buyer, _reason));

        true
    }

    /// Resolve a `Disputed` order. If `release_to_seller` is `true`, pays
    /// the seller (minus platform fee) and marks the order `Completed`;
    /// otherwise refunds the buyer in full and marks it `Refunded`. Emits a
    /// `("dispute", "resolve")` event.
    ///
    /// Returns `true` on success.
    ///
    /// # Panics
    /// - `"not admin"` — `admin` does not match the stored contract admin.
    /// - `"order not found"` — no order with `order_id` exists.
    /// - `"order not in dispute"` — the order is not in `Disputed` status.
    ///
    /// # Auth
    /// Requires authorization from the contract admin.
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

    /// Leave feedback (rating 1-5, comment, completion proof) on a
    /// `Completed` order. Marks the feedback `is_verified` when
    /// `completion_proof` is non-zero. Updates `UserFeedbackReceived` for
    /// the seller and `UserFeedbackGiven` for the reviewer, and emits a
    /// `("feedback", "submit")` event.
    ///
    /// Returns the newly assigned `feedback_id`.
    ///
    /// # Panics
    /// - `"rating must be between 1 and 5"` — `rating` is out of range.
    /// - `"order not found"` — no order with `order_id` exists.
    /// - `"only buyer can leave feedback"` — `reviewer` is not the order's
    ///   buyer.
    /// - `"order not completed yet"` — the order is not in `Completed`
    ///   status.
    /// - `"feedback already submitted"` — feedback already exists for this
    ///   order.
    ///
    /// # Auth
    /// Requires authorization from `reviewer`.
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

    /// Retrieve the full `Listing` record for `listing_id`.
    ///
    /// # Panics
    /// Panics with `"listing not found"` if no listing with `listing_id`
    /// exists.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
    pub fn get_listing(env: Env, listing_id: u64) -> Listing {
        env.storage().instance().get(&DataKey::Listing(listing_id)).expect("listing not found")
    }

    /// Return every listing whose `is_active` flag is `true`.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
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

    /// Retrieve the full `Order` record for `order_id`.
    ///
    /// # Panics
    /// Panics with `"order not found"` if no order with `order_id` exists.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
    pub fn get_order(env: Env, order_id: u64) -> Order {
        env.storage().instance().get(&DataKey::Order(order_id)).expect("order not found")
    }

    /// Return every order placed by `buyer`, in the order they were
    /// created. Returns an empty `Vec` if `buyer` has never purchased
    /// anything.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
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

    /// Return every order received by `seller`, in the order they were
    /// created. Returns an empty `Vec` if `seller` has never sold anything.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
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

    /// Retrieve the full `Feedback` record for `feedback_id`.
    ///
    /// # Panics
    /// Panics with `"feedback not found"` if no feedback with `feedback_id`
    /// exists.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
    pub fn get_feedback(env: Env, feedback_id: u64) -> Feedback {
        env.storage().instance().get(&DataKey::Feedback(feedback_id)).expect("feedback not found")
    }

    /// Return every feedback entry `user` has received as a seller.
    /// Returns an empty `Vec` if `user` has received no feedback yet.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
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

    /// Return `(average_rating, feedback_count)` for `user` as a seller,
    /// computed from every feedback entry received. Returns `(0, 0)` for a
    /// user with no feedback, without panicking.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
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

    /// Return every active listing in `category`.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
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

    /// Update a listing's `price` and/or `is_active` flag. Each `Option`
    /// argument is applied only if `Some`; `new_price` is silently ignored
    /// if it is below `MinListingPrice`. Updates `updated_at` to the
    /// current ledger timestamp.
    ///
    /// Returns `true` on success.
    ///
    /// # Panics
    /// - `"listing not found"` — no listing with `listing_id` exists.
    /// - `"not listing owner"` — `provider` is not the listing's owner.
    ///
    /// # Auth
    /// Requires authorization from `provider`.
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

    /// Compute aggregate `ProviderStats` for `provider`: total listings,
    /// total orders received, completed/disputed order counts, total
    /// revenue earned (after platform fee) from completed orders, and
    /// average rating (via `get_user_rating`).
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
    pub fn get_provider_stats(env: Env, provider: Address) -> ProviderStats {
        let listing_ids: Vec<u64> = env
            .storage().instance().get(&DataKey::ProviderListings(provider.clone())).unwrap_or(Vec::new(&env));
        let total_listings = listing_ids.len() as u32;

        let order_ids: Vec<u64> = env
            .storage().instance().get(&DataKey::SellerOrders(provider.clone())).unwrap_or(Vec::new(&env));
        let total_orders = order_ids.len() as u32;

        let mut completed_orders: u32 = 0;
        let mut disputed_orders: u32 = 0;
        let mut total_revenue: i128 = 0;
        let fee_bps: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PlatformFeeBps)
            .unwrap_or(250);
        for i in 0..order_ids.len() {
            let id = order_ids.get(i).unwrap();
            if let Some(order) = env.storage().instance().get::<DataKey, Order>(&DataKey::Order(id)) {
                if order.status == OrderStatus::Completed {
                    completed_orders += 1;
                    let fee = (order.amount * fee_bps as i128) / 10_000;
                    total_revenue += order.amount - fee;
                } else if order.status == OrderStatus::Disputed {
                    disputed_orders += 1;
                }
            }
        }

        let (avg_rating, _) = Self::get_user_rating(env.clone(), provider.clone());

        ProviderStats {
            total_listings,
            total_orders,
            completed_orders,
            disputed_orders,
            avg_rating,
            total_revenue,
        }
    }

    /// Return `(active_listing_count, platform_fee_bps, min_listing_price)`
    /// for the platform as a whole.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
    pub fn get_platform_stats(env: Env) -> (u32, u32, u32) {
        let total_listings = Self::get_active_listings(env.clone()).len() as u32;
        let fee_bps: u32 = env.storage().instance().get(&DataKey::PlatformFeeBps).unwrap_or(250);
        let min_price: i128 = env.storage().instance().get(&DataKey::MinListingPrice).unwrap_or(100);
        (total_listings, fee_bps, min_price as u32)
    }
}
