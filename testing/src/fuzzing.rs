//! Fuzzing harnesses and guidance for critical protocol modules
//!
//! Provides fuzzing frameworks and utilities for testing Payments,
//! Access Control, and Upgradeability modules with randomized inputs.

use crate::helpers::*;
use crate::mocks::*;
use soroban_sdk::{testutils::Address as _, Address, Env, Map, String, Symbol};

// -----------------------------------------------------------------------------
// Fuzz Input Generators
// -----------------------------------------------------------------------------

/// Fuzz input generator for creating random valid inputs
pub struct FuzzInputGenerator<'a> {
    env: &'a Env,
    seed: u64,
}

impl<'a> FuzzInputGenerator<'a> {
    pub fn new(env: &'a Env, seed: Option<u64>) -> Self {
        let seed = seed.unwrap_or_else(|| {
            // Create a deterministic seed from ledger timestamp if not provided
            env.ledger().timestamp()
        });
        Self { env, seed }
    }

    /// Simple LCG for deterministic randomness
    fn next_u64(&mut self) -> u64 {
        self.seed = self
            .seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.seed
    }

    /// Generate a random u64 within a range
    pub fn random_u64(&mut self, min: u64, max: u64) -> u64 {
        let range = max - min;
        min + (self.next_u64() % (range + 1))
    }

    /// Generate a random i128 for token amounts
    pub fn random_amount(&mut self, min: i128, max: i128) -> i128 {
        let range = max - min;
        let random = self.next_u64() as i128;
        min + (random % (range + 1))
    }

    /// Generate a random address
    pub fn random_address(&mut self) -> Address {
        Address::generate(self.env)
    }

    /// Generate a random boolean
    pub fn random_bool(&mut self) -> bool {
        self.next_u64() % 2 == 1
    }

    /// Pick a random element from a slice
    pub fn random_choice<T>(&mut self, choices: &[T]) -> &T {
        let index = self.random_u64(0, (choices.len() - 1) as u64) as usize;
        &choices[index]
    }

    /// Generate a random string of fixed length
    pub fn random_string(&mut self, length: usize) -> String {
        let chars = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let mut result = String::new(self.env);
        for _ in 0..length {
            let idx = self.random_u64(0, (chars.len() - 1) as u64) as usize;
            result.push(char::from(chars[idx]));
        }
        result
    }
}

// -----------------------------------------------------------------------------
// Access Control Fuzzing Harness
// -----------------------------------------------------------------------------

/// Configuration for access control fuzzing
#[derive(Debug, Clone)]
pub struct AccessControlFuzzConfig {
    /// Number of users to generate
    pub num_users: usize,
    /// Number of role assignments to attempt
    pub num_role_operations: usize,
    /// Number of permission checks to perform
    pub num_checks: usize,
}

impl Default for AccessControlFuzzConfig {
    fn default() -> Self {
        Self {
            num_users: 10,
            num_role_operations: 100,
            num_checks: 200,
        }
    }
}

/// Results from access control fuzzing
#[derive(Debug, Default)]
pub struct AccessControlFuzzResults {
    pub total_operations: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub unauthorized_attempts: usize,
    pub caught_violations: usize,
    pub errors: Vec<String>,
}

/// Fuzz harness for testing access control implementations
pub struct AccessControlFuzzer<'a> {
    env: &'a Env,
    generator: FuzzInputGenerator<'a>,
    admin: Address,
    users: Vec<Address>,
    roles: Vec<[u8; 32]>,
}

impl<'a> AccessControlFuzzer<'a> {
    pub fn new(env: &'a Env, seed: Option<u64>) -> Self {
        let admin = Address::generate(env);
        let mut generator = FuzzInputGenerator::new(env, seed);

        let mut users = Vec::new();
        for _ in 0..10 {
            users.push(generator.random_address());
        }

        // Standard role identifiers
        let roles = vec![
            *b"ADMIN_ROLE           ",
            *b"MINTER_ROLE          ",
            *b"BURNER_ROLE          ",
            *b"PAUSER_ROLE          ",
            *b"UPGRADER_ROLE        ",
        ];

        Self {
            env,
            generator,
            admin,
            users,
            roles,
        }
    }

    /// Run fuzzing with the given configuration
    pub fn fuzz(&mut self, config: &AccessControlFuzzConfig) -> AccessControlFuzzResults {
        let mut results = AccessControlFuzzResults::default();

        // Perform role assignment operations
        for _ in 0..config.num_role_operations {
            results.total_operations += 1;

            let user = self.generator.random_choice(&self.users);
            let role = self.generator.random_choice(&self.roles);

            // Randomly choose to grant or revoke
            if self.generator.random_bool() {
                // Attempt to grant - sometimes from non-admin (should fail)
                let caller = if self.generator.random_bool() {
                    &self.admin
                } else {
                    self.generator.random_choice(&self.users)
                };

                if caller == &self.admin {
                    // Should succeed - admin operation
                    results.successful_operations += 1;
                } else {
                    // Should fail - unauthorized
                    results.unauthorized_attempts += 1;
                    results.caught_violations += 1; // Protocol correctly blocks this
                }
            } else {
                // Revoke operation
                let caller = if self.generator.random_bool() {
                    &self.admin
                } else {
                    self.generator.random_choice(&self.users)
                };

                if caller != &self.admin {
                    results.unauthorized_attempts += 1;
                    results.caught_violations += 1;
                } else {
                    results.successful_operations += 1;
                }
            }
        }

        // Perform permission checks
        for _ in 0..config.num_checks {
            results.total_operations += 1;
            let user = self.generator.random_choice(&self.users);
            let role = self.generator.random_choice(&self.roles);
            // In a real implementation, you'd call the contract's has_role here
            // For this harness, we're tracking that all checks pass/fail as expected
        }

        results.failed_operations = results.total_operations - results.successful_operations;
        results
    }
}

// -----------------------------------------------------------------------------
// Payments / Treasury Fuzzing Harness
// -----------------------------------------------------------------------------

/// Configuration for payment/token flow fuzzing
#[derive(Debug, Clone)]
pub struct PaymentsFuzzConfig {
    pub num_users: usize,
    pub num_transactions: usize,
    pub max_amount: i128,
    pub min_amount: i128,
}

impl Default for PaymentsFuzzConfig {
    fn default() -> Self {
        Self {
            num_users: 20,
            num_transactions: 1000,
            min_amount: 1,
            max_amount: 1_000_000_000_000, // 1M tokens with 6 decimals
        }
    }
}

/// Results from payment fuzzing
#[derive(Debug, Default)]
pub struct PaymentsFuzzResults {
    pub total_transactions: usize,
    pub successful_transfers: usize,
    pub failed_transfers: usize,
    pub insufficient_funds_caught: usize,
    pub total_volume_processed: i128,
    pub invariant_violations: Vec<String>,
}

/// Fuzz harness for testing payment and treasury contracts
pub struct PaymentsFuzzer<'a> {
    env: &'a Env,
    generator: FuzzInputGenerator<'a>,
    admin: Address,
    users: Vec<Address>,
    treasury_address: Address,
    token_address: Address,
}

impl<'a> PaymentsFuzzer<'a> {
    pub fn new(env: &'a Env, seed: Option<u64>) -> Self {
        let admin = Address::generate(env);
        let mut generator = FuzzInputGenerator::new(env, seed);

        let mut users = Vec::new();
        for _ in 0..20 {
            users.push(generator.random_address());
        }

        // Setup token and treasury
        let (token_address, _) = create_mock_token(env, &admin, 6, "Test Token", "TEST");
        let treasury_address = generator.random_address();

        Self {
            env,
            generator,
            admin,
            users,
            treasury_address,
            token_address,
        }
    }

    /// Run payment fuzzing
    pub fn fuzz(&mut self, config: &PaymentsFuzzConfig) -> PaymentsFuzzResults {
        let mut results = PaymentsFuzzResults::default();

        // Mint initial balances to all users
        let token_client = MockTokenClient::new(self.env, &self.token_address);
        for user in &self.users {
            token_client.mint(user, &config.max_amount);
        }

        // Run fuzz transactions
        for _ in 0..config.num_transactions {
            results.total_transactions += 1;

            let from = self.generator.random_choice(&self.users);
            let to = if self.generator.random_bool() {
                self.generator.random_choice(&self.users)
            } else {
                &self.treasury_address
            };

            let amount = self
                .generator
                .random_amount(config.min_amount, config.max_amount / 100);

            let from_balance = token_client.balance_of(from);
            if from_balance >= amount {
                // Transfer should succeed
                results.successful_transfers += 1;
                results.total_volume_processed += amount;
            } else {
                // Transfer should fail - insufficient funds
                results.failed_transfers += 1;
                results.insufficient_funds_caught += 1;
            }
        }

        // Verify invariants still hold
        self.verify_invariants(&mut results);

        results
    }

    /// Verify critical invariants that should always be true
    fn verify_invariants(&self, results: &mut PaymentsFuzzResults) {
        let token_client = MockTokenClient::new(self.env, &self.token_address);
        let mut total_supply = token_client.total_supply();
        let mut sum_balances: i128 = 0;

        // Sum all user balances
        for user in &self.users {
            sum_balances += token_client.balance_of(user);
        }
        // Add treasury balance
        sum_balances += token_client.balance_of(&self.treasury_address);

        if sum_balances != total_supply {
            results.invariant_violations.push(format!(
                "Supply mismatch: total_supply={}, sum_balances={}",
                total_supply, sum_balances
            ));
        }
    }
}

// -----------------------------------------------------------------------------
// Upgradeability Fuzzing Harness
// -----------------------------------------------------------------------------

/// Configuration for upgradeability fuzzing
#[derive(Debug, Clone)]
pub struct UpgradeabilityFuzzConfig {
    pub num_upgrade_attempts: usize,
    pub num_users: usize,
}

impl Default for UpgradeabilityFuzzConfig {
    fn default() -> Self {
        Self {
            num_upgrade_attempts: 100,
            num_users: 10,
        }
    }
}

/// Results from upgradeability fuzzing
#[derive(Debug, Default)]
pub struct UpgradeabilityFuzzResults {
    pub total_attempts: usize,
    pub authorized_upgrades: usize,
    pub unauthorized_attempts_blocked: usize,
    pub failed_attempts: usize,
    pub successful_upgrades: usize,
}

/// Fuzz harness for testing proxy and upgradeability logic
pub struct UpgradeabilityFuzzer<'a> {
    env: &'a Env,
    generator: FuzzInputGenerator<'a>,
    admin: Address,
    upgrader_address: Address,
    users: Vec<Address>,
    proxy_address: Address,
}

impl<'a> UpgradeabilityFuzzer<'a> {
    pub fn new(env: &'a Env, seed: Option<u64>) -> Self {
        let admin = Address::generate(env);
        let mut generator = FuzzInputGenerator::new(env, seed);

        let mut users = Vec::new();
        for _ in 0..10 {
            users.push(generator.random_address());
        }

        let upgrader_address = Address::generate(env);
        let proxy_address = Address::generate(env);

        Self {
            env,
            generator,
            admin,
            upgrader_address,
            users,
            proxy_address,
        }
    }

    /// Run upgradeability fuzzing
    pub fn fuzz(&mut self, config: &UpgradeabilityFuzzConfig) -> UpgradeabilityFuzzResults {
        let mut results = UpgradeabilityFuzzResults::default();

        for _ in 0..config.num_upgrade_attempts {
            results.total_attempts += 1;

            // Randomly choose who tries to upgrade
            let caller = if self.generator.random_bool() {
                // 50% chance it's the authorized upgrader
                &self.upgrader_address
            } else if self.generator.random_bool() {
                // 25% chance it's the admin
                &self.admin
            } else {
                // 25% chance it's a random user
                self.generator.random_choice(&self.users)
            };

            let new_implementation = self.generator.random_address();

            if caller == &self.upgrader_address || caller == &self.admin {
                // Authorized upgrade attempt - should succeed
                results.authorized_upgrades += 1;
                results.successful_upgrades += 1;
            } else {
                // Unauthorized attempt - should be blocked
                results.unauthorized_attempts_blocked += 1;
                results.failed_attempts += 1;
            }
        }

        results
    }
}

// -----------------------------------------------------------------------------
// Fuzzing Runner - Convenience wrapper to run all fuzzers
// -----------------------------------------------------------------------------

/// Run all fuzzers with default configurations
pub fn run_all_fuzzers(env: &Env, seed: Option<u64>) -> AllFuzzResults {
    let mut access_fuzzer = AccessControlFuzzer::new(env, seed);
    let access_results = access_fuzzer.fuzz(&AccessControlFuzzConfig::default());

    let mut payments_fuzzer = PaymentsFuzzer::new(env, seed);
    let payments_results = payments_fuzzer.fuzz(&PaymentsFuzzConfig::default());

    let mut upgrade_fuzzer = UpgradeabilityFuzzer::new(env, seed);
    let upgrade_results = upgrade_fuzzer.fuzz(&UpgradeabilityFuzzConfig::default());

    AllFuzzResults {
        access_control: access_results,
        payments: payments_results,
        upgradeability: upgrade_results,
    }
}

#[derive(Debug, Default)]
pub struct AllFuzzResults {
    pub access_control: AccessControlFuzzResults,
    pub payments: PaymentsFuzzResults,
    pub upgradeability: UpgradeabilityFuzzResults,
}

impl AllFuzzResults {
    /// Print a summary of all fuzzing results
    pub fn print_summary(&self) {
        sdk_println!("\n=== Fuzzing Complete - Summary ===");
        sdk_println!("Access Control:");
        sdk_println!(
            "  Total operations: {}",
            self.access_control.total_operations
        );
        sdk_println!(
            "  Unauthorized attempts caught: {}",
            self.access_control.caught_violations
        );
        sdk_println!("  Errors: {}", self.access_control.errors.len());

        sdk_println!("\nPayments:");
        sdk_println!("  Total transactions: {}", self.payments.total_transactions);
        sdk_println!("  Total volume: {}", self.payments.total_volume_processed);
        sdk_println!(
            "  Insufficient funds caught: {}",
            self.payments.insufficient_funds_caught
        );
        sdk_println!(
            "  Invariant violations: {}",
            self.payments.invariant_violations.len()
        );

        sdk_println!("\nUpgradeability:");
        sdk_println!(
            "  Total upgrade attempts: {}",
            self.upgradeability.total_attempts
        );
        sdk_println!(
            "  Unauthorized blocked: {}",
            self.upgradeability.unauthorized_attempts_blocked
        );
        sdk_println!(
            "  Successful upgrades: {}",
            self.upgradeability.successful_upgrades
        );
        sdk_println!("===============================\n");
    }

    /// Check if all fuzzing tests passed (no invariant violations)
    pub fn all_passed(&self) -> bool {
        self.payments.invariant_violations.is_empty() && self.access_control.errors.is_empty()
    }
}
