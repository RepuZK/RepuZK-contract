use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, String, Vec};

#[contracttype]
#[derive(Clone)]
pub struct Issuer {
    pub address: Address,
    pub name: String,
    pub description: String,
    pub is_active: bool,
    pub registered_at: u64,
    pub updated_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct CredentialType {
    pub id: String,
    pub name: String,
    pub description: String,
    pub schema: String,
    pub requires_zk: bool,
}

#[contracttype]
pub enum DataKey {
    Issuer(Address),
    AllIssuers,
    IssuerCredentialTypes(Address),
    IssuerCount,
    Admin,
}

#[contract]
pub struct IssuerRegistry;

#[contractimpl]
impl IssuerRegistry {
    /// Initialize the contract and set the administrator.
    ///
    /// Must be called exactly once after deployment.  The `admin` address
    /// authorizes the call.
    ///
    /// # Panics
    /// Panics with `"already initialized"` if the contract has already been
    /// initialized.
    ///
    /// # Auth
    /// Requires authorization from `admin`.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }

        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::IssuerCount, &0u32);
    }

    /// Register a new trusted issuer.
    ///
    /// Stores an `Issuer` record with `is_active = true` and the current
    /// ledger timestamp for both `registered_at` and `updated_at`.  The
    /// issuer's address is also appended to the global `AllIssuers` list and
    /// the issuer counter is incremented.
    ///
    /// Returns `true` on success.
    ///
    /// # Panics
    /// Panics with `"issuer already exists"` if `issuer_address` is already
    /// registered.
    ///
    /// # Auth
    /// Requires authorization from the contract admin.
    pub fn add_issuer(
        env: Env,
        issuer_address: Address,
        name: String,
        description: String,
    ) -> bool {
        let admin = Self::get_admin(&env);
        admin.require_auth();

        if env
            .storage()
            .instance()
            .has(&DataKey::Issuer(issuer_address.clone()))
        {
            panic!("issuer already exists");
        }

        let now = env.ledger().timestamp();
        let issuer = Issuer {
            address: issuer_address.clone(),
            name,
            description,
            is_active: true,
            registered_at: now,
            updated_at: now,
        };

        env.storage()
            .instance()
            .set(&DataKey::Issuer(issuer_address.clone()), &issuer);

        let mut issuers: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AllIssuers)
            .unwrap_or(Vec::new(&env));

        issuers.push_back(issuer_address);
        env.storage().instance().set(&DataKey::AllIssuers, &issuers);

        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::IssuerCount)
            .unwrap_or(0);

        env.storage()
            .instance()
            .set(&DataKey::IssuerCount, &(count + 1));

        true
    }

    /// Remove an issuer from the registry entirely.
    ///
    /// Deletes the issuer's storage entry, removes the address from the
    /// `AllIssuers` list, and decrements the issuer counter.
    ///
    /// Returns `true` on success.
    ///
    /// # Panics
    /// Panics with `"issuer does not exist"` if `issuer_address` is not
    /// registered.
    ///
    /// # Auth
    /// Requires authorization from the contract admin.
    pub fn remove_issuer(env: Env, issuer_address: Address) -> bool {
        let admin = Self::get_admin(&env);
        admin.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Issuer(issuer_address.clone()))
        {
            panic!("issuer does not exist");
        }

        env.storage()
            .instance()
            .remove(&DataKey::Issuer(issuer_address.clone()));

        let issuers: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AllIssuers)
            .unwrap_or(Vec::new(&env));

        let mut new_issuers = Vec::new(&env);
        for i in 0..issuers.len() {
            if issuers.get(i).unwrap() != issuer_address {
                new_issuers.push_back(issuers.get(i).unwrap());
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::AllIssuers, &new_issuers);

        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::IssuerCount)
            .unwrap_or(0);

        if count > 0 {
            env.storage()
                .instance()
                .set(&DataKey::IssuerCount, &(count - 1));
        }

        true
    }

    /// Return `true` if `address` is a registered *and active* issuer.
    ///
    /// Returns `false` when the address is unknown or when the issuer has
    /// been deactivated via `update_issuer_status`.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
    pub fn is_issuer(env: Env, address: Address) -> bool {
        let key = DataKey::Issuer(address);
        match env.storage().instance().get::<DataKey, Issuer>(&key) {
            Some(issuer) => issuer.is_active,
            None => false,
        }
    }

    /// Retrieve the full `Issuer` record for `address`.
    ///
    /// # Panics
    /// Panics with `"issuer not found"` if the address is not registered.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
    pub fn get_issuer(env: Env, address: Address) -> Issuer {
        env.storage()
            .instance()
            .get(&DataKey::Issuer(address))
            .expect("issuer not found")
    }

    /// Activate or deactivate an issuer without removing it.
    ///
    /// Sets `is_active` to `is_active` and updates `updated_at` to the
    /// current ledger timestamp.  Deactivated issuers are rejected by the
    /// Reputation Registry when new proofs are submitted.
    ///
    /// Returns `true` on success.
    ///
    /// # Panics
    /// Panics with `"issuer not found"` if the address is not registered.
    ///
    /// # Auth
    /// Requires authorization from the contract admin.
    pub fn update_issuer_status(env: Env, issuer_address: Address, is_active: bool) -> bool {
        let admin = Self::get_admin(&env);
        admin.require_auth();

        let mut issuer: Issuer = env
            .storage()
            .instance()
            .get(&DataKey::Issuer(issuer_address.clone()))
            .expect("issuer not found");

        issuer.is_active = is_active;
        issuer.updated_at = env.ledger().timestamp();

        env.storage()
            .instance()
            .set(&DataKey::Issuer(issuer_address), &issuer);

        true
    }

    /// Register a credential schema for an issuer.
    ///
    /// Appends a new `CredentialType` to the issuer's credential list.
    /// The `schema` field is a JSON-schema string that downstream clients
    /// can use to validate credential payloads.  Set `requires_zk = true`
    /// to signal that proofs of this type must be accompanied by a ZK proof.
    ///
    /// Returns `true` on success.
    ///
    /// # Panics
    /// Panics with `"issuer is not active"` if the issuer exists but has been
    /// deactivated.
    ///
    /// # Auth
    /// Requires authorization from `issuer_address`.
    pub fn register_credential_type(
        env: Env,
        issuer_address: Address,
        credential_id: String,
        name: String,
        description: String,
        schema: String,
        requires_zk: bool,
    ) -> bool {
        issuer_address.require_auth();

        let issuer = Self::get_issuer(env.clone(), issuer_address.clone());
        if !issuer.is_active {
            panic!("issuer is not active");
        }

        let credential = CredentialType {
            id: credential_id.clone(),
            name,
            description,
            schema,
            requires_zk,
        };

        let mut credential_types: Vec<CredentialType> = env
            .storage()
            .instance()
            .get(&DataKey::IssuerCredentialTypes(issuer_address.clone()))
            .unwrap_or(Vec::new(&env));

        credential_types.push_back(credential);

        env.storage().instance().set(
            &DataKey::IssuerCredentialTypes(issuer_address),
            &credential_types,
        );

        true
    }

    /// Return the list of credential types registered by `issuer_address`.
    ///
    /// Returns an empty `Vec` when the issuer has not registered any
    /// credential types yet.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
    pub fn get_issuer_credential_types(env: Env, issuer_address: Address) -> Vec<CredentialType> {
        env.storage()
            .instance()
            .get(&DataKey::IssuerCredentialTypes(issuer_address))
            .unwrap_or(Vec::new(&env))
    }

    /// Return all registered issuers, both active and inactive.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
    pub fn get_all_issuers(env: Env) -> Vec<Issuer> {
        let issuer_addresses: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AllIssuers)
            .unwrap_or(Vec::new(&env));

        let mut issuers = Vec::new(&env);

        for i in 0..issuer_addresses.len() {
            let address = issuer_addresses.get(i).unwrap();
            if let Some(issuer) = env.storage().instance().get(&DataKey::Issuer(address)) {
                issuers.push_back(issuer);
            }
        }

        issuers
    }

    /// Return only the issuers whose `is_active` flag is `true`.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
    pub fn get_active_issuers(env: Env) -> Vec<Issuer> {
        let all_issuers = Self::get_all_issuers(env.clone());
        let mut active_issuers = Vec::new(&env);

        for i in 0..all_issuers.len() {
            let issuer = all_issuers.get(i).unwrap();
            if issuer.is_active {
                active_issuers.push_back(issuer);
            }
        }

        active_issuers
    }

    /// Return the total number of registered issuers (active + inactive).
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
    pub fn get_issuer_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::IssuerCount)
            .unwrap_or(0)
    }

    /// Return `true` if `issuer_address` has registered a credential type
    /// with the given `credential_id`.
    ///
    /// # Auth
    /// No authorization required — anyone may call this.
    pub fn verify_credential_type(
        env: Env,
        issuer_address: Address,
        credential_id: String,
    ) -> bool {
        let credential_types = Self::get_issuer_credential_types(env, issuer_address);

        for i in 0..credential_types.len() {
            let credential = credential_types.get(i).unwrap();
            if credential.id == credential_id {
                return true;
            }
        }

        false
    }

    /// Transfer the admin role to a new address.
    ///
    /// After this call the previous admin loses all privileged access and only
    /// `new_admin` can perform admin-gated operations.
    ///
    /// Returns `true` on success.
    ///
    /// # Auth
    /// Requires authorization from the current admin.
    pub fn transfer_admin(env: Env, new_admin: Address) -> bool {
        let current_admin = Self::get_admin(&env);
        current_admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &new_admin);
        true
    }

    fn get_admin(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("admin not set")
    }

    /// Record the issuance of a credential to a user.
    ///
    /// Stores a mapping `(issuer_address, user_address, credential_id) →
    /// credential_data_hash` in persistent storage.  If `expires_at > 0` the
    /// entry's TTL is extended to `expires_at` ledger sequences.
    ///
    /// Returns `true` on success.
    ///
    /// # Panics
    /// - `"issuer is not active"` — issuer has been deactivated.
    /// - `"credential type not registered"` — the credential type has not
    ///   been registered by this issuer via `register_credential_type`.
    ///
    /// # Auth
    /// Requires authorization from `issuer_address`.
    pub fn issue_credential(
        env: Env,
        issuer_address: Address,
        user_address: Address,
        credential_id: String,
        credential_data_hash: BytesN<32>,
        expires_at: u32,
    ) -> bool {
        issuer_address.require_auth();

        let issuer = Self::get_issuer(env.clone(), issuer_address.clone());
        if !issuer.is_active {
            panic!("issuer is not active");
        }

        if !Self::verify_credential_type(env.clone(), issuer_address.clone(), credential_id.clone())
        {
            panic!("credential type not registered");
        }

        let credential_key = (issuer_address, user_address, credential_id);
        env.storage()
            .persistent()
            .set(&credential_key, &credential_data_hash);

        if expires_at > 0 {
            env.storage()
                .persistent()
                .extend_ttl(&credential_key, expires_at, expires_at);
        }

        true
    }
}
