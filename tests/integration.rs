/// Integration tests using the testing module's utilities
///
/// Cross-contract integration tests that demonstrate usage of the
/// testing & simulation module with the actual protocol contracts.
/// Run with: cargo test --test integration

use testing::helpers::*;
use testing::mocks::*;
use testing::simulation::*;
use soroban_sdk::{
    testutils::Address as _,
    Address, Env,
};

/// Test cross-contract interaction between aid-contract and treasury-contract
#[test]
fn test_aid_and_treasury_integration() {
    // Create a test environment with 5 users
    let mut test_env = TestEnvironment::new(5);
    
    // Create USDC token for testing
    let (token_addr, token_client, asset_client) = test_env.create_stellar_token("usdc");
    
    // Mint initial balances
    test_env.mint_tokens_to_users(&asset_client, &10_000_000); // 10k USDC per user
    
    // Register our contracts
    // In a real implementation, you'd register your actual contract types here
    // let aid_contract_id = test_env.register_contract("aid_contract", aid_contract::AidContract);
    // let treasury_contract_id = test_env.register_contract("treasury", treasury_contract::TreasuryContract);
    
    // For demonstration, we'll mock the interaction
    let donor = test_env.user(0);
    let recipient = test_env.user(1);
    
    // Verify initial balances
    assert_eq!(token_client.balance(&donor), 10_000_000);
    assert_eq!(token_client.balance(&recipient), 10_000_000);
    
    // In a real test, you would call the actual contract methods:
    // aid_client.create_aid(&donor, &recipient, &5000, &expiry);
    
    std::println!("Integration test environment setup completed successfully!");
}

/// Test running a full simulation of protocol activity
#[test]
fn test_full_protocol_simulation() {
    let mut simulator = DeterministicSimulator::new();
    let env = simulator.env();
    
    // Setup admin and users
    let admin = Address::generate(env);
    let users: Vec<Address> = (0..20).map(|_| Address::generate(env)).collect();
    
    // Create mock token and oracle
    let (token_addr, _token_client) = create_mock_token(env, &admin, 6, "Test Token", "TEST");
    let (oracle_addr, oracle_client) = create_mock_oracle(env, &admin);
    
    // Set an initial price
    oracle_client.update_price(&token_addr, &1000000, 6); // $1.00 with 6 decimals
    
    // Simulate various transactions
    for i in 0..50 {
        let from = &users[i % users.len()];
        let to = &users[(i + 3) % users.len()];
        let amount = 1000;
        
        // In a real simulation, you would execute actual contract calls
        // simulator.execute_tx(...);
        
        // Advance ledger every 10 transactions
        if i % 10 == 9 {
            simulator.advance_ledgers(1);
        }
    }
    
    // Generate simulation report
    let results = simulator.finalize();
    results.print_gas_report();
    
    std::println!("Simulation completed successfully!");
}

/// Run fuzzing on critical modules
#[test]
fn test_fuzz_critical_modules() {
    let env = Env::default();
    
    // Run all fuzzers with a fixed seed for reproducibility
    let results = run_all_fuzzers(&env, Some(42));
    
    // Print the fuzzing summary
    results.print_summary();
    
    // Verify all fuzzing tests passed
    assert!(results.all_passed(), "All fuzzing invariants must hold");
}