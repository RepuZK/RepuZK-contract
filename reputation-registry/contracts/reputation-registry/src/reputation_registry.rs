#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, String, Vec, BytesN, Map, 
    Symbol
};

// ==================== Data Structures ====================

#[contracttype]
#[derive(Clone)]
pub struct ReputationProof {
    /// The user who owns this proof
    pub owner: Address,
    /// The issuer who issued the credential
    pub issuer: Address,
    /// Hash of the ZK proof
    pub proof_hash: BytesN<32>,
    /// Hash of the credential data (off-chain)
    pub credential_hash: BytesN<32>,
    /// Type of credential (e.g., "jobs_completed", "success_rate")
    pub credential_type: String,
    /// Timestamp when proof was registered
    pub registered_at: u64,
    /// Timestamp when proof expires (0 = never)
    pub expires_at: u64,
    /// Whether the proof is still valid
    pub is_active: bool,
    /// Additional metadata URI
    pub metadata_uri: String,
}

#[contracttype]
#[derive(Clone)]
pub struct ReputationScore {
    /// Overall reputation score (0-1000)
    pub score: u32,
    /// Timestamp when score was calculated
    pub calculated_at: u64,
    /// Individual score components
    pub components: Map<String, u32>,
    /// Number of proofs contributing to score
    pub proof_count: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct VerificationRequest {
    /// Who requested verification
    pub requester: Address,
    /// Who is being verified
    pub target: Address,
    /// Proof being verified
    pub proof_hash: BytesN<32>,
    /// Timestamp of request
    pub requested_at: u64,
    /// Whether verification was completed
    pub is_verified: bool,
    /// Timestamp of verification
    pub verified_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct ReputationBadge {
    /// Badge ID (e.g., "top_developer", "verified_auditor")
    pub badge_id: String,
    /// Name of the badge
    pub name: String,
    /// Description of requirements
    pub description: String,
    /// Required score threshold
    pub score_threshold: u32,
    /// Required credential types
    pub required_credentials: Vec<String>,
    /// Whether badge is active
    pub is_active: bool,
}

// ==================== Storage Keys ====================

#[contracttype]
pub enum DataKey {
    // User's proofs (Address -> Vec<ProofHash>)
    UserProofs(Address),
    // Individual proof data (ProofHash -> ReputationProof)
    ProofData(BytesN<32>),
    // User's reputation score (Address -> ReputationScore)
    UserScore(Address),
    // Verification requests (request_id -> VerificationRequest)
    VerificationRequest(u64),
    // Reputation badges (badge_id -> ReputationBadge)
    ReputationBadge(String),
    // All badge IDs
    AllBadges,
    // Users who earned specific badge (badge_id -> Vec<Address>)
    BadgeHolders(String),
    // User's badges (Address -> Vec<String>)
    UserBadges(Address),
    // Contract admin
    Admin,
    // Issuer Registry contract address
    IssuerRegistry,
    // Next verification request ID
    NextRequestId,
    // Total proofs registered
    TotalProofs,
    // Proof verification count
    VerificationCount,
}

// ==================== Main Contract ====================

#[contract]
pub struct ReputationRegistry;

#[contractimpl]
impl ReputationRegistry {
    // ============ Initialization ============
    
    pub fn initialize(env: Env, admin: Address, issuer_registry: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        
        admin.require_auth();
        
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::IssuerRegistry, &issuer_registry);
        env.storage().instance().set(&DataKey::NextRequestId, &0u64);
        env.storage().instance().set(&DataKey::TotalProofs, &0u32);
        env.storage().instance().set(&DataKey::VerificationCount, &0u32);
    }
    
    // ============ Proof Management ============
    
    /// Register a new reputation proof
    pub fn register_proof(
        env: Env,
        owner: Address,
        issuer: Address,
        proof_hash: BytesN<32>,
        credential_hash: BytesN<32>,
        credential_type: String,
        expires_at: u64,
        metadata_uri: String,
    ) -> bool {
        // Verify owner authorization
        owner.require_auth();
        
        // Verify issuer is registered and active
        Self::verify_issuer(&env, issuer.clone());
        
        // Check if proof already exists
        if env.storage().instance().has(&DataKey::ProofData(proof_hash.clone())) {
            panic!("proof already registered");
        }
        
        let now = env.ledger().timestamp();
        
        let proof = ReputationProof {
            owner: owner.clone(),
            issuer: issuer.clone(),
            proof_hash: proof_hash.clone(),
            credential_hash,
            credential_type: credential_type.clone(),
            registered_at: now,
            expires_at,
            is_active: true,
            metadata_uri,
        };
        
        // Store proof data
        env.storage().instance().set(&DataKey::ProofData(proof_hash.clone()), &proof);
        
        // Add to user's proofs list
        let mut user_proofs: Vec<BytesN<32>> = env
            .storage()
            .instance()
            .get(&DataKey::UserProofs(owner.clone()))
            .unwrap_or(Vec::new(&env));
        
        user_proofs.push_back(proof_hash.clone());
        env.storage().instance().set(&DataKey::UserProofs(owner.clone()), &user_proofs);
        
        // Update total proofs count
        let total: u32 = env.storage().instance().get(&DataKey::TotalProofs).unwrap_or(0);
        env.storage().instance().set(&DataKey::TotalProofs, &(total + 1));
        
        // Emit event using tuple data (topics: (Symbol, Symbol), data: tuple)
        let topics = (Symbol::new(&env, "proof"), Symbol::new(&env, "reg"));
        let data = (owner.clone(), issuer, proof_hash, credential_type, now);
        env.events().publish(topics, data);
        
        // Update user's reputation score
        Self::update_reputation_score(&env, owner);
        
        true
    }
    
    /// Update an existing proof
    pub fn update_proof(
        env: Env,
        owner: Address,
        proof_hash: BytesN<32>,
        new_credential_hash: BytesN<32>,
        new_expires_at: u64,
        new_metadata_uri: String,
    ) -> bool {
        owner.require_auth();
        
        let mut proof: ReputationProof = env
            .storage()
            .instance()
            .get(&DataKey::ProofData(proof_hash.clone()))
            .expect("proof not found");
        
        // Verify ownership
        if proof.owner != owner {
            panic!("not proof owner");
        }
        
        // Update proof fields
        proof.credential_hash = new_credential_hash;
        proof.expires_at = new_expires_at;
        proof.metadata_uri = new_metadata_uri;
        
        env.storage().instance().set(&DataKey::ProofData(proof_hash), &proof);
        
        // Update reputation score
        Self::update_reputation_score(&env, owner);
        
        true
    }
    
    /// Revoke a proof (issuer or owner can revoke)
    pub fn revoke_proof(env: Env, proof_hash: BytesN<32>, revoker: Address) -> bool {
        revoker.require_auth();
        
        let mut proof: ReputationProof = env
            .storage()
            .instance()
            .get(&DataKey::ProofData(proof_hash.clone()))
            .expect("proof not found");
        
        // Check if revoker is owner or issuer
        if proof.owner != revoker && proof.issuer != revoker {
            // Check if revoker is admin
            let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
            if revoker != admin {
                panic!("not authorized to revoke");
            }
        }
        
        proof.is_active = false;
        env.storage().instance().set(&DataKey::ProofData(proof_hash), &proof);
        
        // Update user's reputation score
        Self::update_reputation_score(&env, proof.owner.clone());
        
        true
    }
    
    /// Get proof by hash
    pub fn get_proof(env: Env, proof_hash: BytesN<32>) -> ReputationProof {
        env.storage()
            .instance()
            .get(&DataKey::ProofData(proof_hash))
            .expect("proof not found")
    }
    
    /// Get all proofs for a user
    pub fn get_user_proofs(env: Env, user: Address) -> Vec<ReputationProof> {
        let proof_hashes: Vec<BytesN<32>> = env
            .storage()
            .instance()
            .get(&DataKey::UserProofs(user.clone()))
            .unwrap_or(Vec::new(&env));
        
        let mut proofs = Vec::new(&env);
        
        for i in 0..proof_hashes.len() {
            let hash = proof_hashes.get(i).unwrap();
            if let Some(proof) = env.storage().instance().get(&DataKey::ProofData(hash)) {
                proofs.push_back(proof);
            }
        }
        
        proofs
    }
    
    /// Get active proofs for a user (not expired and active)
    pub fn get_active_user_proofs(env: Env, user: Address) -> Vec<ReputationProof> {
        let all_proofs = Self::get_user_proofs(env.clone(), user);
        let now = env.ledger().timestamp();
        let mut active_proofs = Vec::new(&env);
        
        for i in 0..all_proofs.len() {
            let proof = all_proofs.get(i).unwrap();
            if proof.is_active && (proof.expires_at == 0 || proof.expires_at > now) {
                active_proofs.push_back(proof);
            }
        }
        
        active_proofs
    }
    
    // ============ Reputation Score Management ============
    
    /// Calculate and update user's reputation score
    fn update_reputation_score(env: &Env, user: Address) {
        let active_proofs = Self::get_active_user_proofs(env.clone(), user.clone());
        
        let mut total_score: u32 = 0;
        let mut components = Map::new(env);
        
        // Calculate score based on proof types
        for i in 0..active_proofs.len() {
            let proof = active_proofs.get(i).unwrap();
            
            // Convert String to str for comparison
            let credential_type_str = proof.credential_type.to_string();
            
            // Base score depends on credential type
            let base_score = match credential_type_str.as_str() {
                "jobs_completed" => 50,
                "success_rate" => 70,
                "contributions" => 40,
                "proposals" => 45,
                "course_completed" => 30,
                "verified_human" => 50,
                _ => 20,
            };
            
            total_score += base_score;
            
            // Track component scores
            let component_key = proof.credential_type.clone();
            let current = components.get(component_key.clone()).unwrap_or(0);
            components.set(component_key, current + base_score);
        }
        
        // Cap score at 1000
        if total_score > 1000 {
            total_score = 1000;
        }
        
        let reputation_score = ReputationScore {
            score: total_score,
            calculated_at: env.ledger().timestamp(),
            components,
            proof_count: active_proofs.len() as u32,
        };
        
        env.storage().instance().set(&DataKey::UserScore(user.clone()), &reputation_score);
        
        // Emit event using tuple data
        let topics = (Symbol::new(env, "score"), Symbol::new(env, "upd"));
        let data = (user, total_score, env.ledger().timestamp());
        env.events().publish(topics, data);
    }
    
    /// Get user's reputation score
    pub fn get_reputation_score(env: Env, user: Address) -> ReputationScore {
        env.storage()
            .instance()
            .get(&DataKey::UserScore(user))
            .unwrap_or(ReputationScore {
                score: 0,
                calculated_at: 0,
                components: Map::new(&env),
                proof_count: 0,
            })
    }
    
    /// Verify if user meets score threshold
    pub fn verify_score_threshold(env: Env, user: Address, threshold: u32) -> bool {
        let score = Self::get_reputation_score(env, user);
        score.score >= threshold
    }
    
    // ============ Verification System ============
    
    /// Request verification of a proof
    pub fn request_verification(
        env: Env,
        requester: Address,
        target: Address,
        proof_hash: BytesN<32>,
    ) -> u64 {
        requester.require_auth();
        
        // Verify proof exists
        let proof = Self::get_proof(env.clone(), proof_hash.clone());
        
        // Verify proof belongs to target
        if proof.owner != target {
            panic!("proof does not belong to target");
        }
        
        let request_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextRequestId)
            .unwrap_or(0);
        
        let request = VerificationRequest {
            requester,
            target,
            proof_hash,
            requested_at: env.ledger().timestamp(),
            is_verified: false,
            verified_at: 0,
        };
        
        env.storage().instance().set(&DataKey::VerificationRequest(request_id), &request);
        env.storage().instance().set(&DataKey::NextRequestId, &(request_id + 1));
        
        request_id
    }
    
    /// Complete verification (by verifier or admin)
    pub fn complete_verification(
        env: Env,
        request_id: u64,
        verifier: Address,
        is_valid: bool,
    ) -> bool {
        verifier.require_auth();
        
        let mut request: VerificationRequest = env
            .storage()
            .instance()
            .get(&DataKey::VerificationRequest(request_id))
            .expect("request not found");
        
        if request.is_verified {
            panic!("already verified");
        }
        
        request.is_verified = is_valid;
        request.verified_at = env.ledger().timestamp();
        
        env.storage().instance().set(&DataKey::VerificationRequest(request_id), &request);
        
        // Update verification count
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::VerificationCount)
            .unwrap_or(0);
        env.storage().instance().set(&DataKey::VerificationCount, &(count + 1));
        
        true
    }
    
    /// Get verification request details
    pub fn get_verification_request(env: Env, request_id: u64) -> VerificationRequest {
        env.storage()
            .instance()
            .get(&DataKey::VerificationRequest(request_id))
            .expect("request not found")
    }
    
    // ============ Badge System ============
    
    /// Create a new reputation badge (admin only)
    pub fn create_badge(
        env: Env,
        badge_id: String,
        name: String,
        description: String,
        score_threshold: u32,
        required_credentials: Vec<String>,
    ) -> bool {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        
        let badge = ReputationBadge {
            badge_id: badge_id.clone(),
            name: name.clone(),
            description,
            score_threshold,
            required_credentials: required_credentials.clone(),
            is_active: true,
        };
        
        env.storage().instance().set(&DataKey::ReputationBadge(badge_id.clone()), &badge);
        
        // Add to all badges list
        let mut all_badges: Vec<String> = env
            .storage()
            .instance()
            .get(&DataKey::AllBadges)
            .unwrap_or(Vec::new(&env));
        
        all_badges.push_back(badge_id);
        env.storage().instance().set(&DataKey::AllBadges, &all_badges);
        
        true
    }
    
    /// Check and award badges to a user
    pub fn check_and_award_badges(env: Env, user: Address) -> Vec<String> {
        user.require_auth();
        
        let user_score = Self::get_reputation_score(env.clone(), user.clone());
        let all_badges: Vec<String> = env
            .storage()
            .instance()
            .get(&DataKey::AllBadges)
            .unwrap_or(Vec::new(&env));
        
        let user_proofs = Self::get_user_proofs(env.clone(), user.clone());
        let mut earned_badges = Vec::new(&env);
        
        for i in 0..all_badges.len() {
            let badge_id = all_badges.get(i).unwrap();
            let badge: ReputationBadge = env
                .storage()
                .instance()
                .get(&DataKey::ReputationBadge(badge_id.clone()))
                .unwrap();
            
            if !badge.is_active {
                continue;
            }
            
            // Check score threshold
            if user_score.score < badge.score_threshold {
                continue;
            }
            
            // Check required credentials
            let mut has_all_credentials = true;
            for j in 0..badge.required_credentials.len() {
                let required_cred = badge.required_credentials.get(j).unwrap();
                let mut found = false;
                
                for k in 0..user_proofs.len() {
                    let proof = user_proofs.get(k).unwrap();
                    if proof.credential_type == required_cred && proof.is_active {
                        found = true;
                        break;
                    }
                }
                
                if !found {
                    has_all_credentials = false;
                    break;
                }
            }
            
            if has_all_credentials {
                // Award badge if not already earned
                let user_badges: Vec<String> = env
                    .storage()
                    .instance()
                    .get(&DataKey::UserBadges(user.clone()))
                    .unwrap_or(Vec::new(&env));
                
                let mut already_has = false;
                for k in 0..user_badges.len() {
                    if user_badges.get(k).unwrap() == badge_id {
                        already_has = true;
                        break;
                    }
                }
                
                if !already_has {
                    // Add to user's badges
                    let mut updated_badges = user_badges;
                    updated_badges.push_back(badge_id.clone());
                    env.storage().instance().set(&DataKey::UserBadges(user.clone()), &updated_badges);
                    
                    // Add to badge holders
                    let mut holders: Vec<Address> = env
                        .storage()
                        .instance()
                        .get(&DataKey::BadgeHolders(badge_id.clone()))
                        .unwrap_or(Vec::new(&env));
                    
                    holders.push_back(user.clone());
                    env.storage().instance().set(&DataKey::BadgeHolders(badge_id.clone()), &holders);
                    
                    earned_badges.push_back(badge_id.clone());
                    
                    // Emit event using tuple data
                    let topics = (Symbol::new(&env, "badge"), Symbol::new(&env, "get"));
                    let data = (user.clone(), badge_id.clone(), badge.name, env.ledger().timestamp());
                    env.events().publish(topics, data);
                }
            }
        }
        
        earned_badges
    }
    
    /// Get user's badges
    pub fn get_user_badges(env: Env, user: Address) -> Vec<ReputationBadge> {
        let badge_ids: Vec<String> = env
            .storage()
            .instance()
            .get(&DataKey::UserBadges(user))
            .unwrap_or(Vec::new(&env));
        
        let mut badges = Vec::new(&env);
        
        for i in 0..badge_ids.len() {
            let badge_id = badge_ids.get(i).unwrap();
            if let Some(badge) = env.storage().instance().get(&DataKey::ReputationBadge(badge_id)) {
                badges.push_back(badge);
            }
        }
        
        badges
    }
    
    /// Get all available badges
    pub fn get_all_badges(env: Env) -> Vec<ReputationBadge> {
        let badge_ids: Vec<String> = env
            .storage()
            .instance()
            .get(&DataKey::AllBadges)
            .unwrap_or(Vec::new(&env));
        
        let mut badges = Vec::new(&env);
        
        for i in 0..badge_ids.len() {
            let badge_id = badge_ids.get(i).unwrap();
            if let Some(badge) = env.storage().instance().get(&DataKey::ReputationBadge(badge_id)) {
                badges.push_back(badge);
            }
        }
        
        badges
    }
    
    // ============ Query Functions ============
    
    /// Get total number of registered proofs
    pub fn get_total_proofs(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::TotalProofs).unwrap_or(0)
    }
    
    /// Get total verification count
    pub fn get_verification_count(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::VerificationCount).unwrap_or(0)
    }
    
    /// Get top users by reputation score (requires external indexing)
    pub fn get_top_users(_env: Env, _limit: u32) -> Vec<(Address, u32)> {
        // Note: This would require external indexing for efficiency
        // Returns empty for now - implement with events and off-chain indexing
        Vec::new(&_env)
    }
    
    // ============ Admin Functions ============
    
    /// Update issuer registry address
    pub fn update_issuer_registry(env: Env, new_registry: Address) -> bool {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        
        env.storage().instance().set(&DataKey::IssuerRegistry, &new_registry);
        true
    }
    
    /// Transfer admin role
    pub fn transfer_admin(env: Env, new_admin: Address) -> bool {
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        current_admin.require_auth();
        
        env.storage().instance().set(&DataKey::Admin, &new_admin);
        true
    }
    
    /// Batch revoke proofs from a specific issuer
    pub fn revoke_issuer_proofs(_env: Env, _issuer: Address) -> u32 {
        // This would require indexing all proofs by issuer
        // For now, returns 0 - implement with proper indexing
        0
    }
    
    // ============ Helper Functions ============
    
    /// Verify issuer is registered and active
    fn verify_issuer(_env: &Env, _issuer: Address) {
        // Note: This requires calling the IssuerRegistry contract
        // Implement cross-contract call here
        // In production: call issuer_registry.is_issuer(issuer)
    }
}