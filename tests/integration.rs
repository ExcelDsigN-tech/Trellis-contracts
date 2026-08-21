#![cfg(test)]

/// Integration tests for the Batch Operations module.
///
/// These tests exercise the batch executor in a more realistic setting,
/// using a Stellar Asset Contract (SAC) as the token and verifying
/// end-to-end token movement.
///
/// Run with: `cargo test --test integration`
extern crate std;

use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Env};

use shared::batch::{
    execute_multi_transfer, multi_transfer_all, BatchConfig, BatchError, BatchMode, BatchTransfer,
    OperationResult,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup_token<'a>(
    env: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = env.register_stellar_asset_contract(admin.clone());
    let client = token::Client::new(env, &contract_address);
    let asset_client = token::StellarAssetClient::new(env, &contract_address);
    (contract_address, client, asset_client)
}

// ---------------------------------------------------------------------------
// Scenario: Distribute aid to multiple recipients in one atomic batch
// ---------------------------------------------------------------------------

#[test]
fn integration_aid_distribution_atomic() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, token_client, asset_client) = setup_token(&env, &admin);

    let donor = Address::generate(&env);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    let r4 = Address::generate(&env);
    let r5 = Address::generate(&env);
    asset_client.mint(&donor, &10_000);

    let config = BatchConfig {
        mode: BatchMode::Atomic,
        max_operations: 10,
    };

    let transfers = Vec::from_array(
        &env,
        [
            BatchTransfer {
                to: r1.clone(),
                amount: 500,
            },
            BatchTransfer {
                to: r2.clone(),
                amount: 500,
            },
            BatchTransfer {
                to: r3.clone(),
                amount: 500,
            },
            BatchTransfer {
                to: r4.clone(),
                amount: 500,
            },
            BatchTransfer {
                to: r5.clone(),
                amount: 500,
            },
        ],
    );

    let result = execute_multi_transfer(&env, &donor, &token_addr, &transfers, &config).unwrap();

    assert_eq!(result.total, 5);
    assert_eq!(result.succeeded, 5);
    assert_eq!(result.failed, 0);
    assert!(!result.reverted);

    // Each recipient got 500 tokens.
    assert_eq!(token_client.balance(&r1), 500);
    assert_eq!(token_client.balance(&r2), 500);
    assert_eq!(token_client.balance(&r3), 500);
    assert_eq!(token_client.balance(&r4), 500);
    assert_eq!(token_client.balance(&r5), 500);
    // Donor retained 10_000 - 2_500 = 7_500.
    assert_eq!(token_client.balance(&donor), 7_500);
}

// ---------------------------------------------------------------------------
// Scenario: Partial failure in non-atomic mode preserves successful transfers
// ---------------------------------------------------------------------------

#[test]
fn integration_partial_failure_preserves_successes() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, token_client, asset_client) = setup_token(&env, &admin);

    let sender = Address::generate(&env);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    // Enough for exactly 2 of 3 transfers (200 + 200 = 400).
    asset_client.mint(&sender, &400);

    let config = BatchConfig {
        mode: BatchMode::NonAtomic,
        max_operations: 10,
    };

    let transfers = Vec::from_array(
        &env,
        [
            BatchTransfer {
                to: r1.clone(),
                amount: 200,
            },
            BatchTransfer {
                to: r2.clone(),
                amount: 200,
            },
            BatchTransfer {
                to: r3.clone(),
                amount: 200,
            },
        ],
    );

    let result = execute_multi_transfer(&env, &sender, &token_addr, &transfers, &config).unwrap();

    assert_eq!(result.total, 3);
    assert_eq!(result.succeeded, 2);
    assert_eq!(result.failed, 1);
    assert!(!result.reverted);

    // First two transfers went through.
    assert_eq!(token_client.balance(&r1), 200);
    assert_eq!(token_client.balance(&r2), 200);
    // Third failed — no tokens left.
    assert_eq!(token_client.balance(&r3), 0);
    // Sender drained all 400 tokens.
    assert_eq!(token_client.balance(&sender), 0);
}

// ---------------------------------------------------------------------------
// Scenario: Atomic batch with sufficient balance succeeds entirely
// ---------------------------------------------------------------------------

#[test]
fn integration_atomic_batch_all_succeed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, token_client, asset_client) = setup_token(&env, &admin);

    let sender = Address::generate(&env);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    asset_client.mint(&sender, &300);

    let config = BatchConfig {
        mode: BatchMode::Atomic,
        max_operations: 10,
    };

    let transfers = Vec::from_array(
        &env,
        [
            BatchTransfer {
                to: r1.clone(),
                amount: 100,
            },
            BatchTransfer {
                to: r2.clone(),
                amount: 200,
            },
        ],
    );

    let result = execute_multi_transfer(&env, &sender, &token_addr, &transfers, &config).unwrap();

    assert_eq!(result.total, 2);
    assert_eq!(result.succeeded, 2);
    assert_eq!(result.failed, 0);
    assert!(!result.reverted);
    assert_eq!(token_client.balance(&r1), 100);
    assert_eq!(token_client.balance(&r2), 200);
    assert_eq!(token_client.balance(&sender), 0);
}

// ---------------------------------------------------------------------------
// Scenario: Empty and oversized batches are rejected
// ---------------------------------------------------------------------------

#[test]
fn integration_empty_batch_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, _tc, _ac) = setup_token(&env, &admin);
    let sender = Address::generate(&env);

    let config = BatchConfig::atomic_default();
    let transfers = Vec::new(&env);

    let result = execute_multi_transfer(&env, &sender, &token_addr, &transfers, &config);
    assert_eq!(result, Err(BatchError::EmptyBatch));
}

#[test]
fn integration_oversized_batch_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, _tc, ac) = setup_token(&env, &admin);
    let sender = Address::generate(&env);
    ac.mint(&sender, &100_000);

    let config = BatchConfig {
        mode: BatchMode::Atomic,
        max_operations: 3,
    };
    let r = Address::generate(&env);
    let transfers = Vec::from_array(
        &env,
        [
            BatchTransfer {
                to: r.clone(),
                amount: 10,
            },
            BatchTransfer {
                to: r.clone(),
                amount: 10,
            },
            BatchTransfer {
                to: r.clone(),
                amount: 10,
            },
            BatchTransfer {
                to: r.clone(),
                amount: 10,
            },
        ],
    );

    let result = execute_multi_transfer(&env, &sender, &token_addr, &transfers, &config);
    assert_eq!(result, Err(BatchError::BatchTooLarge));
}

// ---------------------------------------------------------------------------
// Scenario: multi_transfer_all convenience helper
// ---------------------------------------------------------------------------

#[test]
fn integration_multi_transfer_all_end_to_end() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, token_client, asset_client) = setup_token(&env, &admin);

    let sender = Address::generate(&env);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    let r4 = Address::generate(&env);
    asset_client.mint(&sender, &1_000);

    let recipients = [
        (r1.clone(), 100_i128),
        (r2.clone(), 200_i128),
        (r3.clone(), 300_i128),
        (r4.clone(), 400_i128),
    ];

    let result = multi_transfer_all(&env, &sender, &token_addr, &recipients).unwrap();

    assert_eq!(result.total, 4);
    assert_eq!(result.succeeded, 4);
    assert!(!result.reverted);

    assert_eq!(token_client.balance(&r1), 100);
    assert_eq!(token_client.balance(&r2), 200);
    assert_eq!(token_client.balance(&r3), 300);
    assert_eq!(token_client.balance(&r4), 400);
    assert_eq!(token_client.balance(&sender), 0);
}

// ---------------------------------------------------------------------------
// Scenario: Reject invalid amounts (zero / negative)
// ---------------------------------------------------------------------------

#[test]
fn integration_zero_amount_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, _tc, ac) = setup_token(&env, &admin);
    let sender = Address::generate(&env);
    let r = Address::generate(&env);
    ac.mint(&sender, &1000);

    let config = BatchConfig::atomic_default();
    let transfers = Vec::from_array(&env, [BatchTransfer { to: r, amount: 0 }]);

    let result = execute_multi_transfer(&env, &sender, &token_addr, &transfers, &config);
    assert_eq!(result, Err(BatchError::InvalidOperation));
}

#[test]
fn integration_negative_amount_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, _tc, ac) = setup_token(&env, &admin);
    let sender = Address::generate(&env);
    let r = Address::generate(&env);
    ac.mint(&sender, &1000);

    let config = BatchConfig::atomic_default();
    let transfers = Vec::from_array(
        &env,
        [BatchTransfer {
            to: r,
            amount: -100,
        }],
    );

    let result = execute_multi_transfer(&env, &sender, &token_addr, &transfers, &config);
    assert_eq!(result, Err(BatchError::InvalidOperation));
}

// ---------------------------------------------------------------------------
// Scenario: Result structure consistency
// ---------------------------------------------------------------------------

#[test]
fn integration_result_fields_are_consistent() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, _tc, ac) = setup_token(&env, &admin);
    let sender = Address::generate(&env);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    ac.mint(&sender, &10_000);

    let config = BatchConfig {
        mode: BatchMode::Atomic,
        max_operations: 5,
    };
    let transfers = Vec::from_array(
        &env,
        [
            BatchTransfer {
                to: r1,
                amount: 100,
            },
            BatchTransfer {
                to: r2,
                amount: 200,
            },
            BatchTransfer {
                to: r3,
                amount: 300,
            },
        ],
    );

    let result = execute_multi_transfer(&env, &sender, &token_addr, &transfers, &config).unwrap();

    // total == succeeded + failed (for a non-reverted batch)
    assert_eq!(result.total, result.succeeded + result.failed);
    // results vector length matches total
    assert_eq!(result.results.len() as u32, result.total);
    // All results are Success
    for i in 0..result.results.len() {
        assert_eq!(result.results.get_unchecked(i), OperationResult::Success);
    }
}

// ---------------------------------------------------------------------------
// Scenario: max_operations boundary — exactly at limit succeeds
// ---------------------------------------------------------------------------

#[test]
fn integration_batch_at_exact_limit_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, _tc, ac) = setup_token(&env, &admin);
    let sender = Address::generate(&env);
    ac.mint(&sender, &5_000);

    let max = 5u32;
    let config = BatchConfig {
        mode: BatchMode::Atomic,
        max_operations: max,
    };

    let transfers = Vec::from_array(
        &env,
        [
            BatchTransfer {
                to: Address::generate(&env),
                amount: 100,
            },
            BatchTransfer {
                to: Address::generate(&env),
                amount: 100,
            },
            BatchTransfer {
                to: Address::generate(&env),
                amount: 100,
            },
            BatchTransfer {
                to: Address::generate(&env),
                amount: 100,
            },
            BatchTransfer {
                to: Address::generate(&env),
                amount: 100,
            },
        ],
    );

    let result = execute_multi_transfer(&env, &sender, &token_addr, &transfers, &config).unwrap();

    assert_eq!(result.total, 5);
    assert_eq!(result.succeeded, 5);
    assert!(!result.reverted);
}

// ---------------------------------------------------------------------------
// Scenario: Single transfer in batch behaves like a normal transfer
// ---------------------------------------------------------------------------

#[test]
fn integration_single_transfer_in_batch() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, token_client, asset_client) = setup_token(&env, &admin);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    asset_client.mint(&sender, &500);

    let config = BatchConfig::atomic_default();
    let transfers = Vec::from_array(
        &env,
        [BatchTransfer {
            to: recipient.clone(),
            amount: 300,
        }],
    );

    let result = execute_multi_transfer(&env, &sender, &token_addr, &transfers, &config).unwrap();

    assert_eq!(result.total, 1);
    assert_eq!(result.succeeded, 1);
    assert_eq!(result.failed, 0);
    assert!(!result.reverted);
    assert_eq!(token_client.balance(&recipient), 300);
    assert_eq!(token_client.balance(&sender), 200);
}

// ---------------------------------------------------------------------------
// Scenario: Invalid config is rejected
// ---------------------------------------------------------------------------

#[test]
fn integration_invalid_config_zero_max_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, _tc, ac) = setup_token(&env, &admin);
    let sender = Address::generate(&env);
    ac.mint(&sender, &1000);

    let config = BatchConfig {
        mode: BatchMode::Atomic,
        max_operations: 0,
    };
    let transfers = Vec::from_array(
        &env,
        [BatchTransfer {
            to: Address::generate(&env),
            amount: 100,
        }],
    );

    let result = execute_multi_transfer(&env, &sender, &token_addr, &transfers, &config);
    assert_eq!(result, Err(BatchError::InvalidConfig));
}

#[test]
fn integration_invalid_config_exceeds_absolute_max() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_addr, _tc, ac) = setup_token(&env, &admin);
    let sender = Address::generate(&env);
    ac.mint(&sender, &1000);

    let config = BatchConfig {
        mode: BatchMode::Atomic,
        max_operations: 101, // exceeds ABSOLUTE_MAX_BATCH_SIZE (100)
    };
    let transfers = Vec::from_array(
        &env,
        [BatchTransfer {
            to: Address::generate(&env),
            amount: 100,
        }],
    );

    let result = execute_multi_transfer(&env, &sender, &token_addr, &transfers, &config);
    assert_eq!(result, Err(BatchError::InvalidConfig));
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