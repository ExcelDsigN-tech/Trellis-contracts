//! Mock implementations for external dependencies
//!
//! Provides mock contracts and clients for tokens, oracles, and other external
//! systems commonly used across the protocol.

use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env, String, Map,
};

// -----------------------------------------------------------------------------
// Mock Token Implementation
// -----------------------------------------------------------------------------

/// Mock ERC20-like token contract for testing
#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn initialize(env: Env, admin: Address, decimals: u32, name: String, symbol: String) {
        if !env.storage().instance().has(&b"initialized") {
            env.storage().instance().set(&b"admin", &admin);
            env.storage().instance().set(&b"decimals", &decimals);
            env.storage().instance().set(&b"name", &name);
            env.storage().instance().set(&b"symbol", &symbol);
            env.storage().instance().set(&b"initialized", &true);
            env.storage().instance().set(b"total_supply", &0i128);
        }
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        let admin: Address = env.storage().instance().get(&b"admin").unwrap();
        admin.require_auth();
        
        let mut balance: i128 = env.storage().persistent().get(&to).unwrap_or(0);
        balance += amount;
        env.storage().persistent().set(&to, &balance);
        
        let mut total_supply: i128 = env.storage().instance().get(b"total_supply").unwrap();
        total_supply += amount;
        env.storage().instance().set(b"total_supply", &total_supply);
    }

    pub fn transfer(env: Env, to: Address, amount: i128) -> bool {
        let from = env.current_contract_address();
        let mut from_balance: i128 = env.storage().persistent().get(&from).unwrap_or(0);
        let mut to_balance: i128 = env.storage().persistent().get(&to).unwrap_or(0);
        
        if from_balance >= amount {
            from_balance -= amount;
            to_balance += amount;
            env.storage().persistent().set(&from, &from_balance);
            env.storage().persistent().set(&to, &to_balance);
            true
        } else {
            false
        }
    }

    pub fn balance_of(env: Env, owner: Address) -> i128 {
        env.storage().persistent().get(&owner).unwrap_or(0)
    }

    pub fn total_supply(env: Env) -> i128 {
        env.storage().instance().get(b"total_supply").unwrap()
    }

    pub fn decimals(env: Env) -> u32 {
        env.storage().instance().get(&b"decimals").unwrap()
    }
}

/// Mock Token client for easy testing
pub struct MockTokenClient<'a> {
    env: &'a Env,
    address: Address,
}

impl<'a> MockTokenClient<'a> {
    pub fn new(env: &'a Env, address: &Address) -> Self {
        Self { env, address: address.clone() }
    }

    pub fn mint(&self, to: &Address, amount: i128) {
        self.env.invoke_contract(
            &self.address,
            &Symbol::new(self.env, "mint"),
            (to.clone(), amount),
        );
    }

    pub fn balance_of(&self, owner: &Address) -> i128 {
        self.env.invoke_contract(
            &self.address,
            &Symbol::new(self.env, "balance_of"),
            (owner.clone(),)
        )
    }
}

// -----------------------------------------------------------------------------
// Mock Oracle Implementation
// -----------------------------------------------------------------------------

/// Price data structure for oracle
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[soroban_sdk::contracttype]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
    pub decimals: u32,
}

/// Mock Price Oracle contract
#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    pub fn initialize(env: Env, admin: Address) {
        if !env.storage().instance().has(&b"oracle_initialized") {
            env.storage().instance().set(&b"admin", &admin);
            env.storage().instance().set(&b"oracle_initialized", &true);
        }
    }

    pub fn update_price(env: Env, asset: Address, price: i128, decimals: u32) {
        let admin: Address = env.storage().instance().get(&b"admin").unwrap();
        admin.require_auth();

        let timestamp = env.ledger().timestamp();
        let price_data = PriceData {
            price,
            timestamp,
            decimals,
        };
        env.storage().persistent().set(&asset, &price_data);
    }

    pub fn get_price(env: Env, asset: Address) -> PriceData {
        env.storage().persistent().get(&asset).unwrap_or_default()
    }

    pub fn get_last_updated(env: Env, asset: Address) -> u64 {
        let price_data: PriceData = env.storage().persistent().get(&asset).unwrap_or_default();
        price_data.timestamp
    }
}

// -----------------------------------------------------------------------------
// Mock Registry Implementation
// -----------------------------------------------------------------------------

/// Mock Registry contract for testing cross-contract dependencies
#[contract]
pub struct MockRegistry;

#[contractimpl]
impl MockRegistry {
    pub fn initialize(env: Env, admin: Address) {
        if !env.storage().instance().has(&b"registry_initialized") {
            env.storage().instance().set(&b"admin", &admin);
            env.storage().instance().set(&b"registry_initialized", &true);
        }
    }

    pub fn register_contract(env: Env, name: String, address: Address) {
        let admin: Address = env.storage().instance().get(&b"admin").unwrap();
        admin.require_auth();
        env.storage().instance().set(&name, &address);
    }

    pub fn get_contract(env: Env, name: String) -> Option<Address> {
        env.storage().instance().get(&name)
    }
}

// -----------------------------------------------------------------------------
// Factory functions to easily create mocks in tests
// -----------------------------------------------------------------------------

/// Create and initialize a mock token
pub fn create_mock_token(
    env: &Env,
    admin: &Address,
    decimals: u32,
    name: &str,
    symbol: &str,
) -> (Address, MockTokenClient) {
    let token_address = env.register_contract(None, MockToken);
    let client = MockTokenClient::new(env, &token_address);
    
    let name_str = String::from_str(env, name);
    let symbol_str = String::from_str(env, symbol);
    
    env.invoke_contract(
        &token_address,
        &Symbol::new(env, "initialize"),
        (admin.clone(), decimals, name_str, symbol_str),
    );
    
    (token_address, client)
}

/// Create and initialize a mock oracle
pub fn create_mock_oracle(env: &Env, admin: &Address) -> (Address, MockOracle) {
    let oracle_address = env.register_contract(None, MockOracle);
    env.invoke_contract(
        &oracle_address,
        &Symbol::new(env, "initialize"),
        (admin.clone(),),
    );
    (oracle_address, MockOracle)
}

/// Create and initialize a mock registry
pub fn create_mock_registry(env: &Env, admin: &Address) -> Address {
    let registry_address = env.register_contract(None, MockRegistry);
    env.invoke_contract(
        &registry_address,
        &Symbol::new(env, "initialize"),
        (admin.clone(),),
    );
    registry_address
}