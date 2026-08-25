//! Test helpers for time manipulation, tracing, and common testing utilities
//!
//! Provides helper functions for ledger manipulation, auth mocking, and
//! common test setup patterns.

use core::fmt::Write;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env, Map, String, Symbol,
};

// -----------------------------------------------------------------------------
// Time / Ledger Manipulation Helpers
// -----------------------------------------------------------------------------

/// Advance the ledger sequence number by a specific delta
pub fn advance_ledger_sequence(env: &Env, delta: u32) {
    env.ledger().with_mut(|ledger| {
        ledger.sequence_number += delta;
    });
}

/// Advance the ledger timestamp by a specific number of seconds
pub fn advance_ledger_time(env: &Env, seconds: u64) {
    env.ledger().with_mut(|ledger| {
        ledger.timestamp += seconds;
    });
}

/// Jump to a specific ledger sequence number
pub fn set_ledger_sequence(env: &Env, sequence: u32) {
    env.ledger().with_mut(|ledger| {
        ledger.sequence_number = sequence;
    });
}

/// Jump to a specific timestamp
pub fn set_ledger_timestamp(env: &Env, timestamp: u64) {
    env.ledger().with_mut(|ledger| {
        ledger.timestamp = timestamp;
    });
}

/// Reset ledger to genesis state (for fresh test starts)
pub fn reset_ledger_to_genesis(env: &Env) {
    env.ledger().with_mut(|ledger| {
        ledger.sequence_number = 1;
        ledger.timestamp = 1640995200; // 2022-01-01T00:00:00Z
        ledger.network_passphrase = "Test SDF Network ; September 2015".to_string();
    });
}

/// Get the current ledger sequence
pub fn current_ledger_sequence(env: &Env) -> u32 {
    env.ledger().sequence()
}

/// Get the current ledger timestamp
pub fn current_ledger_timestamp(env: &Env) -> u64 {
    env.ledger().timestamp()
}

// -----------------------------------------------------------------------------
// Common Test Setup Helpers
// -----------------------------------------------------------------------------

/// Standard test environment setup with common mocks
#[derive(Debug, Clone)]
pub struct TestEnvironment {
    pub env: Env,
    pub admin: Address,
    pub users: Vec<Address>,
    pub contracts: Map<String, Address>,
}

impl TestEnvironment {
    /// Create a new test environment with admin and predefined number of users
    pub fn new(num_users: usize) -> Self {
        let env = Env::default();
        env.mock_all_auths();
        reset_ledger_to_genesis(&env);

        let admin = Address::generate(&env);
        let mut users = Vec::with_capacity(num_users);
        for _ in 0..num_users {
            users.push(Address::generate(&env));
        }

        let mut contracts = Map::new(&env);

        Self {
            env,
            admin,
            users,
            contracts,
        }
    }

    /// Register a contract in the environment
    pub fn register_contract<T>(&mut self, name: &str, contract: T) -> Address {
        let address = self.env.register_contract(None, contract);
        self.contracts
            .set(String::from_str(&self.env, name), address.clone());
        address
    }

    /// Get a registered contract by name
    pub fn get_contract(&self, name: &str) -> Option<Address> {
        self.contracts.get(String::from_str(&self.env, name))
    }

    /// Create and register a Stellar asset token for testing
    pub fn create_stellar_token(
        &mut self,
        name: &str,
    ) -> (Address, token::Client, token::StellarAssetClient) {
        let contract_address = self.env.register_stellar_asset_contract(self.admin.clone());
        let client = token::Client::new(&self.env, &contract_address);
        let asset_client = token::StellarAssetClient::new(&self.env, &contract_address);

        // Mint initial supply to admin for distribution
        asset_client.mint(&self.admin, &0);

        self.contracts
            .set(String::from_str(&self.env, name), contract_address.clone());
        (contract_address, client, asset_client)
    }

    /// Mint tokens to multiple users at once
    pub fn mint_tokens_to_users(
        &self,
        token_asset_client: &token::StellarAssetClient,
        amount: i128,
    ) {
        for user in &self.users {
            token_asset_client.mint(user, &amount);
        }
    }

    /// Get a user by index
    pub fn user(&self, index: usize) -> Address {
        self.users[index].clone()
    }
}

// -----------------------------------------------------------------------------
// Tracing / Debugging Helpers
// -----------------------------------------------------------------------------

/// Simple event logger for tracing contract interactions
#[derive(Default, Debug)]
pub struct EventTracer {
    events: Vec<(u64, String, Vec<String>)>,
}

impl EventTracer {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Log an event with timestamp
    pub fn log_event(&mut self, timestamp: u64, event_name: &str, data: &[&str]) {
        let data_str: Vec<String> = data.iter().map(|s| s.to_string()).collect();
        self.events
            .push((timestamp, event_name.to_string(), data_str));
    }

    /// Print all events in order
    pub fn print_events(&self) {
        for (ts, name, data) in &self.events {
            let data_str = data.join(", ");
            sdk_println!("[{}] {}: {}", ts, name, data_str);
        }
    }

    /// Filter events by name
    pub fn filter_by_name(&self, event_name: &str) -> Vec<&(u64, String, Vec<String>)> {
        self.events
            .iter()
            .filter(|(_, name, _)| name == event_name)
            .collect()
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

/// Helper to format addresses for logging
pub fn format_address(address: &Address) -> String {
    format!("{:?}", address)
}

/// Simple print macro for test debugging
#[macro_export]
macro_rules! sdk_println {
    ($($arg:tt)*) => {
        #[cfg(feature = "std")]
        std::println!($($arg)*);
    };
}

// -----------------------------------------------------------------------------
// Authentication Helpers
// -----------------------------------------------------------------------------

/// Enable all auth mocking (bypasses all require_auth checks)
pub fn enable_all_auths(env: &Env) {
    env.mock_all_auths();
}

/// Mock auth for specific addresses only
pub fn mock_auths_for_addresses(env: &Env, addresses: &[Address]) {
    for addr in addresses {
        env.mock_auths(&[addr.clone()]);
    }
}

// -----------------------------------------------------------------------------
// Assertion Helpers
// -----------------------------------------------------------------------------

/// Assert that a balance change is as expected
pub fn assert_balance_change(
    env: &Env,
    token_client: &token::Client,
    account: &Address,
    before: i128,
    expected_change: i128,
) {
    let after = token_client.balance(account);
    let actual_change = after - before;
    assert_eq!(
        actual_change, expected_change,
        "Balance change mismatch: expected {}, got {}",
        expected_change, actual_change
    );
}

/// Assert that an event was emitted
pub fn assert_event_emitted<F>(env: &Env, predicate: F) -> bool
where
    F: Fn(&soroban_sdk::Event) -> bool,
{
    env.events().all().iter().any(predicate)
}

/// Get count of specific events
pub fn count_events<F>(env: &Env, predicate: F) -> usize
where
    F: Fn(&soroban_sdk::Event) -> bool,
{
    env.events().all().iter().filter(predicate).count()
}
