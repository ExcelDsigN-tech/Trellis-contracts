//! Example test suites demonstrating usage of the testing module
//!
//! Contains example integration tests that show how to use all the features
//! of the testing & simulation module with real contract patterns.

#![cfg(test)]
extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};
use crate::helpers::*;
use crate::mocks::*;
use crate::simulation::*;
use crate::fuzzing::*;

// -----------------------------------------------------------------------------
// Example Integration Test - Treasury Contract Workflow
// -----------------------------------------------------------------------------

#[test]
fn example_treasury_deposit_withdraw_workflow() {
    // Create test environment using the testing module's TestEnvironment
    let mut test_env = TestEnvironment::new(5); // 5 users
    
    // Create a token for testing
    let (token_addr, token_client, asset_client) = test_env.create_stellar_token("usdc");
    
    // Mint initial balances to all users
    test_env.mint_tokens_to_users(&asset_client, &10_000_000); // 10k USDC for each user
    
    // Register a mock treasury contract (in real usage, this would be your actual treasury contract)
    let treasury_addr = test_env.register_contract("treasury", MockTreasury);
    let treasury_client = MockTreasuryClient::new(&test_env.env, &treasury_addr);
    
    // Initialize the treasury
    treasury_client.initialize(&test_env.admin, &token_addr);
    
    // Record pre-deposit balances
    let user0 = test_env.user(0);
    let balance_before = token_client.balance(&user0);
    
    // User 0 deposits 1000 USDC
    treasury_client.deposit(&user0, &1000);
    
    // Verify balance changed correctly
    assert_balance_change(&test_env.env, &token_client, &user0, balance_before, -1000);
    assert_eq!(token_client.balance(&treasury_addr), 1000);
    
    // Advance time 1 day (86400 seconds)
    advance_ledger_time(&test_env.env, 86400);
    
    // User 0 withdraws 500 USDC
    let balance_before_withdraw = token_client.balance(&user0);
    treasury_client.withdraw(&user0, &500);
    
    // Verify withdrawal worked
    assert_balance_change(&test_env.env, &token_client, &user0, balance_before_withdraw, 500);
    assert_eq!(token_client.balance(&treasury_addr), 500);
    
    std::println!("Treasury workflow test completed successfully!");
}

// -----------------------------------------------------------------------------
// Example Simulation Test - Multi-user Payment Scenario
// -----------------------------------------------------------------------------

#[test]
fn example_simulation_multi_user_payments() {
    // Create a deterministic simulator
    let mut simulator = DeterministicSimulator::new();
    let env = simulator.env();
    
    // Setup participants
    let admin = Address::generate(env);
    let users: Vec<Address> = (0..10).map(|_| Address::generate(env)).collect();
    
    // Create token
    let (token_addr, mut token_client, asset_client) = env.register_stellar_asset_contract_with_client(admin.clone());
    
    // Mint to all users
    for user in &users {
        asset_client.mint(user, &1_000_000);
    }
    
    // Run simulation of many transactions
    for i in 0..100 {
        let from = &users[i % users.len()];
        let to = &users[(i + 1) % users.len()];
        let amount = 1000;
        
        // Execute transfer in simulation
        let _ = simulator.execute_tx::<()>(
            &format!("transfer_{}", i),
            &token_addr,
            "transfer",
            from,
            (to.clone(), amount).into(),
        );
        
        // Every 10 transactions, advance a ledger
        if i % 10 == 9 {
            simulator.advance_ledgers(1);
        }
    }
    
    // Generate and print simulation report
    let results = simulator.finalize();
    results.print_gas_report();
    
    assert!(results.failed_txs().is_empty(), "All transactions should succeed");
}

// -----------------------------------------------------------------------------
// Example Access Control Fuzzing Test
// -----------------------------------------------------------------------------

#[test]
fn example_fuzz_access_control() {
    let env = Env::default();
    
    // Run fuzzing with a fixed seed for reproducibility
    let mut fuzzer = AccessControlFuzzer::new(&env, Some(12345));
    let results = fuzzer.fuzz(&AccessControlFuzzConfig {
        num_users: 5,
        num_role_operations: 50,
        num_checks: 100,
    });
    
    results.print_summary();
    
    // Verify all unauthorized attempts were correctly caught
    assert!(results.caught_violations == results.unauthorized_attempts,
            "All unauthorized attempts must be caught");
}

// -----------------------------------------------------------------------------
// Example Gas Profiler Usage
// -----------------------------------------------------------------------------

#[test]
fn example_gas_profiling_operations() {
    let env = Env::default();
    let mut profiler = GasProfiler::new(&env);
    
    // Simulate measuring various operations
    profiler.record_measurement("initialize", 145000);
    profiler.record_measurement("initialize", 147000);
    profiler.record_measurement("deposit", 95000);
    profiler.record_measurement("deposit", 98000);
    profiler.record_measurement("withdraw", 115000);
    profiler.record_measurement("withdraw", 118000);
    profiler.record_measurement("transfer", 78000);
    
    // Print comparison
    profiler.print_comparison();
    
    // Verify we can get stats
    let deposit_stats = profiler.get_stats("deposit").unwrap();
    assert_eq!(deposit_stats.count, 2);
}

// -----------------------------------------------------------------------------
// Mock Treasury for example tests (simulating a real treasury contract)
// -----------------------------------------------------------------------------

#[soroban_sdk::contract]
struct MockTreasury;

#[soroban_sdk::contractimpl]
impl MockTreasury {
    pub fn initialize(env: Env, admin: Address, token: Address) {
        env.storage().instance().set(&b"admin", &admin);
        env.storage().instance().set(&b"token", &token);
    }
    
    pub fn deposit(env: Env, from: Address, amount: i128) {
        let token: Address = env.storage().instance().get(&b"token").unwrap();
        let token_client = token::Client::new(&env, &token);
        // In a real contract, this would transfer from the user
        // For the example, we just track the balance change
        let mut treasury_balance: i128 = env.storage().persistent().get(&b"balance").unwrap_or(0);
        treasury_balance += amount;
        env.storage().persistent().set(&b"balance", &treasury_balance);
    }
    
    pub fn withdraw(env: Env, to: Address, amount: i128) {
        let admin: Address = env.storage().instance().get(&b"admin").unwrap();
        to.require_auth();
        
        let mut treasury_balance: i128 = env.storage().persistent().get(&b"balance").unwrap_or(0);
        treasury_balance -= amount;
        env.storage().persistent().set(&b"balance", &treasury_balance);
    }
}

struct MockTreasuryClient<'a> {
    env: &'a Env,
    address: Address,
}

impl<'a> MockTreasuryClient<'a> {
    pub fn new(env: &'a Env, address: &Address) -> Self {
        Self { env, address: address.clone() }
    }
    
    pub fn initialize(&self, admin: &Address, token: &Address) {
        self.env.invoke_contract(
            &self.address,
            &soroban_sdk::Symbol::new(self.env, "initialize"),
            (admin.clone(), token.clone()),
        );
    }
    
    pub fn deposit(&self, from: &Address, amount: i128) {
        self.env.invoke_contract(
            &self.address,
            &soroban_sdk::Symbol::new(self.env, "deposit"),
            (from.clone(), amount),
        );
    }
    
    pub fn withdraw(&self, to: &Address, amount: i128) {
        self.env.invoke_contract(
            &self.address,
            &soroban_sdk::Symbol::new(self.env, "withdraw"),
            (to.clone(), amount),
        );
    }
}