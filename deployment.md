# RepuZK — Testnet Deployment

**Network:** Stellar Testnet  
**Deployed:** 2026-06-18  
**Admin:** `GBDAMG7J7CMDFPV5ZGCAKOFPUZJ263EWITRTSNBFZEYSBI5H2IR7543R`

---

## Contract Addresses

| Contract | ID |
|---|---|
| IssuerRegistry | `CBKPGRVKOSSLZL3CPLHFMQOUKAFR2HJDSVOVKLNCBBZY5RYPNGI3YE6S` |
| ReputationRegistry | `CA63GY2TWJTKGECG6FPR4ITW4G5PUH3PCGY7P6HY3EC6NM2VSJIATFOK` |
| Marketplace | `CBCUF26JXDAT64BEOWD5GPH5MNX5OAYW7BUYWSPBVJII5DSO67R6O4RE` |

---

## Explorer Links

| Contract | Link |
|---|---|
| IssuerRegistry | https://lab.stellar.org/r/testnet/contract/CBKPGRVKOSSLZL3CPLHFMQOUKAFR2HJDSVOVKLNCBBZY5RYPNGI3YE6S |
| ReputationRegistry | https://lab.stellar.org/r/testnet/contract/CA63GY2TWJTKGECG6FPR4ITW4G5PUH3PCGY7P6HY3EC6NM2VSJIATFOK |
| Marketplace | https://lab.stellar.org/r/testnet/contract/CBCUF26JXDAT64BEOWD5GPH5MNX5OAYW7BUYWSPBVJII5DSO67R6O4RE |

---

## Deployment Transactions

### IssuerRegistry
| Step | Transaction |
|---|---|
| Upload WASM | `75a31fc4b0a1f97e6bfdc48c682b371c18293ca69945f6805ee60b6fc2314605` |
| Deploy | `1ee0df9e7d07bbc627d838e254938cce676707204764af4e59f2ee686ef7ac61` |
| Initialize | `3b4b3332d8fd267749494047e08d0e48643bff6b218bf18924d04f91a52c8087` |

### ReputationRegistry
| Step | Transaction |
|---|---|
| Upload WASM | `40ac665cb81cc7af61800e959d83fa9052fe8cbc437268f805cd21c0010bf632` |
| Deploy | `7934380bb40e254d2245266c2a345df07f455aa19ec3e29ddd773ff22a655e3a` |
| Initialize | `34ed7b88e624bf939a90e4bed4cb633e00fc8f1c60797e572704b0bead8e1040` |

### Marketplace
| Step | Transaction |
|---|---|
| Upload WASM | `e3dfd67a63611784e9c47e4d019bea014520cc5cd33c706f236fd3068f2af30c` |
| Deploy | `fbf5f12e5553df4a926ad592523d4cbd35a0f6486e831d2e71fa90f601c98508` |
| Initialize | `73e5d700007b4a51d883c253ce0d9aa93995a4c65783d3e1df5666029db5c4a1` |

---

## Initialization Parameters

### IssuerRegistry
```
admin: GBDAMG7J7CMDFPV5ZGCAKOFPUZJ263EWITRTSNBFZEYSBI5H2IR7543R
```

### ReputationRegistry
```
admin:           GBDAMG7J7CMDFPV5ZGCAKOFPUZJ263EWITRTSNBFZEYSBI5H2IR7543R
issuer_registry: CBKPGRVKOSSLZL3CPLHFMQOUKAFR2HJDSVOVKLNCBBZY5RYPNGI3YE6S
```

### Marketplace
```
admin:                GBDAMG7J7CMDFPV5ZGCAKOFPUZJ263EWITRTSNBFZEYSBI5H2IR7543R
reputation_registry:  CA63GY2TWJTKGECG6FPR4ITW4G5PUH3PCGY7P6HY3EC6NM2VSJIATFOK
issuer_registry:      CBKPGRVKOSSLZL3CPLHFMQOUKAFR2HJDSVOVKLNCBBZY5RYPNGI3YE6S
platform_fee_bps:     250  (2.5%)
fee_recipient:        GBDAMG7J7CMDFPV5ZGCAKOFPUZJ263EWITRTSNBFZEYSBI5H2IR7543R
```

---

## WASM Hashes

| Contract | WASM Hash |
|---|---|
| IssuerRegistry | `f33caf483cd4f1a1024b805010617a2e46985f11a50c8a1480c780e1d7939494` |
| ReputationRegistry | `85d5e755f3d9e0234fbd633c07e7c107f15192d2d976dbb00698c7cfe3da852d` |
| Marketplace | `5bd72d4c2956cbd681317def83bd6ee6ef9fb1cd4afe47df75d5bb4fb7822d76` |

---

## Backend Environment Variables

```bash
STELLAR_NETWORK=testnet
ISSUER_REGISTRY_CONTRACT=CBKPGRVKOSSLZL3CPLHFMQOUKAFR2HJDSVOVKLNCBBZY5RYPNGI3YE6S
REPUTATION_REGISTRY_CONTRACT=CA63GY2TWJTKGECG6FPR4ITW4G5PUH3PCGY7P6HY3EC6NM2VSJIATFOK
MARKETPLACE_CONTRACT=CBCUF26JXDAT64BEOWD5GPH5MNX5OAYW7BUYWSPBVJII5DSO67R6O4RE
```

## Frontend Environment Variables

```bash
NEXT_PUBLIC_STELLAR_NETWORK=testnet
NEXT_PUBLIC_ISSUER_REGISTRY_CONTRACT=CBKPGRVKOSSLZL3CPLHFMQOUKAFR2HJDSVOVKLNCBBZY5RYPNGI3YE6S
NEXT_PUBLIC_REPUTATION_REGISTRY_CONTRACT=CA63GY2TWJTKGECG6FPR4ITW4G5PUH3PCGY7P6HY3EC6NM2VSJIATFOK
NEXT_PUBLIC_MARKETPLACE_CONTRACT=CBCUF26JXDAT64BEOWD5GPH5MNX5OAYW7BUYWSPBVJII5DSO67R6O4RE
```

---

## Build Notes

- **Rust version:** 1.96.0
- **Target:** `wasm32v1-none` (required for Rust 1.82+; `wasm32-unknown-unknown` is unsupported)
- **Build command:** `stellar contract build` (per contract directory)
- **Fixes applied:**
  - Moved `#![no_std]` from contract source files to each `lib.rs` (crate root)
  - Replaced `soroban_sdk::String::to_string()` with `if/else` comparisons using `String::from_str()` (not available in `no_std`)

## Re-deploying

```bash
# Build
cd issuer-registry && stellar contract build
cd ../reputation-registry && stellar contract build
cd ../marketplace && stellar contract build

# Deploy (in order)
stellar contract deploy --wasm issuer-registry/target/wasm32v1-none/release/issuer_registry.wasm --source <SECRET> --network testnet
stellar contract deploy --wasm reputation-registry/target/wasm32v1-none/release/reputation_registry.wasm --source <SECRET> --network testnet
stellar contract deploy --wasm marketplace/target/wasm32v1-none/release/market.wasm --source <SECRET> --network testnet

# Initialize IssuerRegistry
stellar contract invoke --id <ISSUER_ID> --source <SECRET> --network testnet -- initialize --admin <ADMIN>

# Initialize ReputationRegistry
stellar contract invoke --id <REP_ID> --source <SECRET> --network testnet -- initialize --admin <ADMIN> --issuer_registry <ISSUER_ID>

# Initialize Marketplace
stellar contract invoke --id <MKT_ID> --source <SECRET> --network testnet -- initialize --admin <ADMIN> --reputation_registry <REP_ID> --issuer_registry <ISSUER_ID> --platform_fee_bps 250 --fee_recipient <ADMIN>
```
