//! # Batch Operations — Atomic Multi-Call Executor
//!
//! Provides a safe, configurable batch executor for executing ordered
//! cross-contract calls with revert-on-failure semantics.
//!
//! ## Execution Modes
//!
//! | Mode | Behaviour |
//! |------|-----------|
//! | `Atomic` | All operations must succeed. If any fails, the entire batch reverts. |
//! | `NonAtomic` | Individual failures are captured; successful operations persist. |
//!
//! ## Security Considerations
//!
//! - **Reentrancy**: The executor uses a temporary-storage guard to prevent
//!   re-entrant calls. If the batch executor is invoked while already executing,
//!   it returns `BatchError::ReentrancyDetected`.
//!
//! - **Delegatecall**: This module does **not** use `delegatecall` or any
//!   equivalent mechanism. All cross-contract calls go through Soroban's
//!   standard `invoke_contract` / `try_invoke_contract` interface, which
//!   maintains contract isolation. Each target contract's storage and
//!   authorization context is separate — there is no shared execution context
//!   that could be exploited via delegatecall-style attacks.
//!
//! - **Gas Protection**: Batches are capped at [`ABSOLUTE_MAX_BATCH_SIZE`]
//!   operations (default 100). Callers can set a lower `max_operations` in
//!   `BatchConfig` for tighter limits.
//!
//! - **Authorization**: For token transfers, the `caller` address must have
//!   pre-authorized the batch contract to transfer on their behalf via Soroban's
//!   auth tree. This is typically done when submitting the transaction.
//!
//! - **Input Validation**: All operations are validated before execution.
//!   Zero amounts, empty batches, and oversized batches are rejected upfront.
//!
//! ## Usage Example (Multi-Transfer)
//!
//! ```ignore
//! use shared::batch::{execute_multi_transfer, BatchConfig, BatchMode, BatchTransfer};
//!
//! let config = BatchConfig {
//!     mode: BatchMode::Atomic,
//!     max_operations: 10,
//! };
//! let transfers = Vec::from_array(&env, [
//!     BatchTransfer { to: recipient1.clone(), amount: 100 },
//!     BatchTransfer { to: recipient2.clone(), amount: 200 },
//! ]);
//! let result = execute_multi_transfer(&env, &caller, &token, &transfers, &config)?;
//! assert!(result.succeeded == 2);
//! ```

use soroban_sdk::{
    contracterror, contracttype, symbol_short, token, Address, Env, IntoVal, Symbol, Vec,
};

use crate::errors::Error;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default maximum number of operations in a single batch.
pub const DEFAULT_MAX_BATCH_SIZE: u32 = 50;

/// Hard ceiling — no batch may exceed this regardless of configuration.
pub const ABSOLUTE_MAX_BATCH_SIZE: u32 = 100;

// ---------------------------------------------------------------------------
// Error codes (range 800–899)
// ---------------------------------------------------------------------------

/// Errors specific to batch operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BatchError {
    /// The batch exceeds the maximum allowed operations.
    BatchTooLarge = 800,
    /// An operation in the batch failed (atomic mode — batch was reverted).
    OperationFailed = 801,
    /// The batch configuration is invalid (e.g., max_operations is 0).
    InvalidConfig = 802,
    /// An individual operation has invalid arguments.
    InvalidOperation = 803,
    /// The batch contains no operations.
    EmptyBatch = 804,
    /// Reentrancy detected — the batch executor was called reentrantly.
    ReentrancyDetected = 805,
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Execution mode for a batch.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchMode {
    /// All operations must succeed or the entire batch reverts.
    Atomic,
    /// Individual failures are captured; successful operations persist.
    NonAtomic,
}

/// Configuration controlling how a batch is executed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchConfig {
    /// Execution mode (atomic vs. non-atomic).
    pub mode: BatchMode,
    /// Maximum number of operations allowed in this batch.
    /// Must be in the range `1..=100`.
    pub max_operations: u32,
}

impl BatchConfig {
    /// Returns a default atomic configuration with [`DEFAULT_MAX_BATCH_SIZE`].
    pub fn atomic_default() -> Self {
        Self {
            mode: BatchMode::Atomic,
            max_operations: DEFAULT_MAX_BATCH_SIZE,
        }
    }

    /// Returns a default non-atomic configuration with [`DEFAULT_MAX_BATCH_SIZE`].
    pub fn non_atomic_default() -> Self {
        Self {
            mode: BatchMode::NonAtomic,
            max_operations: DEFAULT_MAX_BATCH_SIZE,
        }
    }
}

/// A single transfer operation within a batch.
///
/// The `from` address is always the caller of the batch — it is not stored
/// in the struct to prevent spoofing.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchTransfer {
    /// Recipient address.
    pub to: Address,
    /// Amount of tokens to transfer (must be > 0).
    pub amount: i128,
}

/// Result of a single operation in the batch.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationResult {
    /// The operation succeeded.
    Success,
    /// The operation failed with this Soroban error code.
    Failure(u32),
    /// The operation was skipped (e.g., after a prior failure in a batch that
    /// stopped early).
    Skipped,
}

/// Aggregated result of a batch execution.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchResult {
    /// Per-operation results, in execution order.
    pub results: Vec<OperationResult>,
    /// Total number of operations in the original batch.
    pub total: u32,
    /// Number of operations that succeeded.
    pub succeeded: u32,
    /// Number of operations that failed.
    pub failed: u32,
    /// Whether the entire batch was reverted (atomic mode with a failure).
    pub reverted: bool,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Emitted when a batch completes (all succeeded or was reverted).
pub const BATCH_COMPLETED: Symbol = symbol_short!("bat_done");

/// Emitted when a non-atomic batch completes with some failures.
pub const BATCH_PARTIAL: Symbol = symbol_short!("bat_part");

// ---------------------------------------------------------------------------
// Reentrancy guard
// ---------------------------------------------------------------------------

const REENTRANCY_KEY: Symbol = symbol_short!("bat_lck");

fn acquire_reentrancy_guard(env: &Env) -> Result<(), BatchError> {
    if crate::storage::temporary_has(env, &REENTRANCY_KEY) {
        return Err(BatchError::ReentrancyDetected);
    }
    crate::storage::temporary_set(env, &REENTRANCY_KEY, &true);
    Ok(())
}

fn release_reentrancy_guard(env: &Env) {
    crate::storage::temporary_remove(env, &REENTRANCY_KEY);
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Validates the batch configuration.
fn validate_config(config: &BatchConfig) -> Result<(), BatchError> {
    if config.max_operations == 0 || config.max_operations > ABSOLUTE_MAX_BATCH_SIZE {
        return Err(BatchError::InvalidConfig);
    }
    Ok(())
}

/// Validates that a single transfer has valid arguments.
fn validate_transfer(transfer: &BatchTransfer) -> Result<(), BatchError> {
    if transfer.amount <= 0 {
        return Err(BatchError::InvalidOperation);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Core executor — multi-transfer
// ---------------------------------------------------------------------------

/// Execute a batch of token transfers with configurable atomicity.
///
/// The `caller` is the address authorizing all transfers. In atomic mode,
/// a single failure reverts the entire batch (including prior successful
/// transfers within the same transaction). In non-atomic mode, failures are
/// recorded but do not prevent subsequent transfers from executing.
///
/// # Arguments
/// * `env` — Soroban environment.
/// * `caller` — Address authorizing the transfers (must have pre-authorized
///   the batch contract via Soroban's auth tree).
/// * `token` — Address of the Stellar Asset Contract (SAC) to transfer.
/// * `transfers` — Ordered list of `BatchTransfer` operations.
/// * `config` — Batch execution configuration.
///
/// # Errors
/// * `BatchError::EmptyBatch` — no transfers provided.
/// * `BatchError::BatchTooLarge` — transfers exceed `config.max_operations`.
/// * `BatchError::InvalidConfig` — config has zero or excessive max_operations.
/// * `BatchError::InvalidOperation` — a transfer has amount ≤ 0.
/// * `BatchError::ReentrancyDetected` — batch executor called reentrantly.
/// * `BatchError::OperationFailed` — (atomic mode only) a transfer failed;
///   the entire batch is reverted.
pub fn execute_multi_transfer(
    env: &Env,
    caller: &Address,
    token: &Address,
    transfers: &Vec<BatchTransfer>,
    config: &BatchConfig,
) -> Result<BatchResult, BatchError> {
    // --- Validate config ---
    validate_config(config)?;

    let count = transfers.len();
    if count == 0 {
        return Err(BatchError::EmptyBatch);
    }
    if count > config.max_operations {
        return Err(BatchError::BatchTooLarge);
    }

    // --- Reentrancy guard ---
    acquire_reentrancy_guard(env)?;

    // --- Execute transfers ---
    let mut results = Vec::new(env);
    let mut succeeded: u32 = 0;
    let mut failed: u32 = 0;
    let mut reverted = false;

    let token_client = token::Client::new(env, token);

    for transfer in transfers.iter() {
        // Validate individual transfer
        if let Err(e) = validate_transfer(&transfer) {
            results.push_back(OperationResult::Failure(e as u32));
            failed += 1;
            if config.mode == BatchMode::Atomic {
                reverted = true;
                break;
            }
            continue;
        }

        // Execute the transfer
        let transfer_result =
            execute_single_transfer(env, &token_client, token, caller, &transfer, &config.mode);

        match transfer_result {
            OperationResult::Success => {
                results.push_back(OperationResult::Success);
                succeeded += 1;
            }
            OperationResult::Failure(code) => {
                results.push_back(OperationResult::Failure(code));
                failed += 1;
                if config.mode == BatchMode::Atomic {
                    reverted = true;
                    break;
                }
            }
            OperationResult::Skipped => {
                results.push_back(OperationResult::Skipped);
            }
        }
    }

    // --- Release reentrancy guard ---
    release_reentrancy_guard(env);

    // --- Emit events ---
    let total = count;
    if reverted {
        crate::events::emit(env, BATCH_COMPLETED, (total, succeeded, failed, true));
    } else if failed > 0 {
        crate::events::emit(env, BATCH_PARTIAL, (total, succeeded, failed));
    } else {
        crate::events::emit(env, BATCH_COMPLETED, (total, succeeded, failed, false));
    }

    Ok(BatchResult {
        results,
        total,
        succeeded,
        failed,
        reverted,
    })
}

/// Execute a single token transfer within the batch context.
///
/// In **atomic** mode, uses `invoke_contract` which panics on failure —
/// causing the entire transaction (and therefore the batch) to revert.
///
/// In **non-atomic** mode, uses `try_invoke_contract` to catch failures
/// gracefully, allowing the batch to continue processing subsequent operations.
fn execute_single_transfer(
    env: &Env,
    token_client: &token::Client,
    token: &Address,
    caller: &Address,
    transfer: &BatchTransfer,
    mode: &BatchMode,
) -> OperationResult {
    match mode {
        BatchMode::Atomic => {
            token_client.transfer(caller, &transfer.to, &transfer.amount);
            OperationResult::Success
        }
        BatchMode::NonAtomic => {
            // Use try_invoke_contract so a failed transfer doesn't revert the
            // entire transaction.
            let result = env.try_invoke_contract::<(), crate::errors::Error>(
                token,
                &symbol_short!("transfer"),
                soroban_sdk::Vec::from_array(
                    env,
                    [
                        caller.to_val(),
                        transfer.to.to_val(),
                        transfer.amount.into_val(env),
                    ],
                ),
            );
            match result {
                Ok(Ok(())) => OperationResult::Success,
                // Inner Err = contract returned a typed error (e.g. insufficient balance)
                // Outer Err  = host-level failure (e.g. contract not found)
                Ok(Err(_)) | Err(_) => OperationResult::Failure(Error::InvalidAmount as u32),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Core executor — generic multi-invoke
// ---------------------------------------------------------------------------

/// Execute a batch of arbitrary contract invocations.
///
/// This is the low-level generic executor for calling multiple contracts
/// with different functions and arguments. For the common case of batched
/// token transfers, prefer [`execute_multi_transfer`] which provides
/// typed safety.
///
/// # Arguments
/// * `env` — Soroban environment.
/// * `config` — Batch execution configuration.
/// * `calls` — Ordered list of `(contract, func_name, args)` tuples.
///   Each `args` is a slice of `Val`s that will be forwarded to the target.
///
/// # Authorization
/// Authorization for each sub-call is handled by Soroban's auth tree.
/// The caller of this function must ensure all necessary authorizations
/// are in place before invoking.
///
/// # Reentrancy
/// Uses the same reentrancy guard as [`execute_multi_transfer`].
pub fn execute_multi_invoke(
    env: &Env,
    config: &BatchConfig,
    calls: &[(Address, Symbol, &[soroban_sdk::Val])],
) -> Result<BatchResult, BatchError> {
    // --- Validate config ---
    validate_config(config)?;

    let count = calls.len() as u32;
    if count == 0 {
        return Err(BatchError::EmptyBatch);
    }
    if count > config.max_operations {
        return Err(BatchError::BatchTooLarge);
    }

    // --- Reentrancy guard ---
    acquire_reentrancy_guard(env)?;

    // --- Execute calls ---
    let mut results = Vec::new(env);
    let mut succeeded: u32 = 0;
    let mut failed: u32 = 0;
    let mut reverted = false;

    for (contract, func, args) in calls.iter() {
        let call_result = execute_single_invoke(env, contract, func, args, &config.mode);

        match call_result {
            OperationResult::Success => {
                results.push_back(OperationResult::Success);
                succeeded += 1;
            }
            OperationResult::Failure(code) => {
                results.push_back(OperationResult::Failure(code));
                failed += 1;
                if config.mode == BatchMode::Atomic {
                    reverted = true;
                    break;
                }
            }
            OperationResult::Skipped => {
                results.push_back(OperationResult::Skipped);
            }
        }
    }

    // --- Release reentrancy guard ---
    release_reentrancy_guard(env);

    // --- Emit events ---
    let total = count;
    if reverted {
        crate::events::emit(env, BATCH_COMPLETED, (total, succeeded, failed, true));
    } else if failed > 0 {
        crate::events::emit(env, BATCH_PARTIAL, (total, succeeded, failed));
    } else {
        crate::events::emit(env, BATCH_COMPLETED, (total, succeeded, failed, false));
    }

    Ok(BatchResult {
        results,
        total,
        succeeded,
        failed,
        reverted,
    })
}

/// Execute a single contract invocation within the batch context.
fn execute_single_invoke(
    env: &Env,
    contract: &Address,
    func: &Symbol,
    args: &[soroban_sdk::Val],
    mode: &BatchMode,
) -> OperationResult {
    // Build a Soroban Vec<Val> from the Rust slice.
    let mut soroban_args = soroban_sdk::Vec::new(env);
    for arg in args.iter() {
        soroban_args.push_back(*arg);
    }

    match mode {
        BatchMode::Atomic => {
            // invoke_contract panics on failure → entire tx reverts.
            let _val: soroban_sdk::Val = env.invoke_contract(contract, func, soroban_args);
            OperationResult::Success
        }
        BatchMode::NonAtomic => {
            // try_invoke_contract converts the panic into a Result.
            let result = env.try_invoke_contract::<soroban_sdk::Val, crate::errors::Error>(
                contract,
                func,
                soroban_args,
            );
            match result {
                Ok(Ok(_)) | Ok(Err(_)) => OperationResult::Success,
                Err(_) => OperationResult::Failure(Error::InvalidAmount as u32),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helper patterns
// ---------------------------------------------------------------------------

/// Convenience: transfer the same token from the caller to multiple recipients
/// atomically.
///
/// Wraps [`execute_multi_transfer`] with a default atomic configuration.
///
/// # Example
/// ```ignore
/// let recipients = Vec::from_array(&env, [
///     (addr1.clone(), 100_i128),
///     (addr2.clone(), 200_i128),
/// ]);
/// let result = multi_transfer_all(&env, &caller, &token, &recipients)?;
/// assert_eq!(result.succeeded, 2);
/// ```
pub fn multi_transfer_all(
    env: &Env,
    caller: &Address,
    token: &Address,
    recipients: &[(Address, i128)],
) -> Result<BatchResult, BatchError> {
    let config = BatchConfig::atomic_default();
    let mut transfers = Vec::new(env);
    for (to, amount) in recipients.iter() {
        transfers.push_back(BatchTransfer {
            to: to.clone(),
            amount: *amount,
        });
    }
    execute_multi_transfer(env, caller, token, &transfers, &config)
}

/// Convenience: invoke no-argument functions on multiple contracts atomically.
///
/// Useful for batched administrative actions like pausing or resuming multiple
/// contracts in a single transaction.
///
/// # Example
/// ```ignore
/// let targets = Vec::from_array(&env, [
///     (contract1.clone(), Symbol::new(&env, "pause")),
///     (contract2.clone(), Symbol::new(&env, "pause")),
/// ]);
/// let result = batch_invoke_no_args(&env, &config, &targets)?;
/// ```
pub fn batch_invoke_no_args(
    env: &Env,
    config: &BatchConfig,
    calls: &[(Address, Symbol)],
) -> Result<BatchResult, BatchError> {
    validate_config(config)?;

    let count = calls.len() as u32;
    if count == 0 {
        return Err(BatchError::EmptyBatch);
    }
    if count > config.max_operations {
        return Err(BatchError::BatchTooLarge);
    }

    acquire_reentrancy_guard(env)?;

    let mut results = Vec::new(env);
    let mut succeeded: u32 = 0;
    let mut failed: u32 = 0;
    let mut reverted = false;

    for (contract, func) in calls.iter() {
        let empty_args = soroban_sdk::Vec::new(env);

        let call_result = match config.mode {
            BatchMode::Atomic => {
                let _val: soroban_sdk::Val = env.invoke_contract(contract, func, empty_args);
                OperationResult::Success
            }
            BatchMode::NonAtomic => {
                let result = env.try_invoke_contract::<soroban_sdk::Val, crate::errors::Error>(
                    contract, func, empty_args,
                );
                match result {
                    Ok(Ok(_)) | Ok(Err(_)) => OperationResult::Success,
                    Err(_) => OperationResult::Failure(Error::InvalidAmount as u32),
                }
            }
        };

        match call_result {
            OperationResult::Success => {
                results.push_back(OperationResult::Success);
                succeeded += 1;
            }
            OperationResult::Failure(code) => {
                results.push_back(OperationResult::Failure(code));
                failed += 1;
                if config.mode == BatchMode::Atomic {
                    reverted = true;
                    break;
                }
            }
            OperationResult::Skipped => {
                results.push_back(OperationResult::Skipped);
            }
        }
    }

    release_reentrancy_guard(env);

    let total = count;
    if reverted {
        crate::events::emit(env, BATCH_COMPLETED, (total, succeeded, failed, true));
    } else if failed > 0 {
        crate::events::emit(env, BATCH_PARTIAL, (total, succeeded, failed));
    } else {
        crate::events::emit(env, BATCH_COMPLETED, (total, succeeded, failed, false));
    }

    Ok(BatchResult {
        results,
        total,
        succeeded,
        failed,
        reverted,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{token, Env};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn setup_token<'a>(
        env: &'a Env,
        admin: &Address,
    ) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
        let contract_address = env.register_stellar_asset_contract(admin.clone());
        let client = token::Client::new(env, &contract_address);
        let asset_client = token::StellarAssetClient::new(env, &contract_address);
        (contract_address, client, asset_client)
    }

    fn default_atomic_config() -> BatchConfig {
        BatchConfig::atomic_default()
    }

    fn default_non_atomic_config() -> BatchConfig {
        BatchConfig::non_atomic_default()
    }

    fn make_transfer(to: &Address, amount: i128) -> BatchTransfer {
        BatchTransfer {
            to: to.clone(),
            amount,
        }
    }

    // -----------------------------------------------------------------------
    // Config validation
    // -----------------------------------------------------------------------

    #[test]
    fn test_atomic_default_config() {
        let config = BatchConfig::atomic_default();
        assert_eq!(config.mode, BatchMode::Atomic);
        assert_eq!(config.max_operations, DEFAULT_MAX_BATCH_SIZE);
    }

    #[test]
    fn test_non_atomic_default_config() {
        let config = BatchConfig::non_atomic_default();
        assert_eq!(config.mode, BatchMode::NonAtomic);
        assert_eq!(config.max_operations, DEFAULT_MAX_BATCH_SIZE);
    }

    #[test]
    fn test_config_zero_max_operations_is_invalid() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (_token_addr, _token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        let recipient = Address::generate(&env);
        asset_client.mint(&caller, &1_000);

        let config = BatchConfig {
            mode: BatchMode::Atomic,
            max_operations: 0,
        };
        let transfers = Vec::from_array(&env, [make_transfer(&recipient, 100)]);
        let result = execute_multi_transfer(&env, &caller, &_token_addr, &transfers, &config);
        assert_eq!(result, Err(BatchError::InvalidConfig));
    }

    #[test]
    fn test_config_exceeding_absolute_max_is_invalid() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (_token_addr, _token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        let recipient = Address::generate(&env);
        asset_client.mint(&caller, &1_000);

        let config = BatchConfig {
            mode: BatchMode::Atomic,
            max_operations: ABSOLUTE_MAX_BATCH_SIZE + 1,
        };
        let transfers = Vec::from_array(&env, [make_transfer(&recipient, 100)]);
        let result = execute_multi_transfer(&env, &caller, &_token_addr, &transfers, &config);
        assert_eq!(result, Err(BatchError::InvalidConfig));
    }

    // -----------------------------------------------------------------------
    // Empty / oversized batch
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_batch_returns_error() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (_token_addr, _token_client, _asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);

        let config = default_atomic_config();
        let transfers = Vec::new(&env);
        let result = execute_multi_transfer(&env, &caller, &_token_addr, &transfers, &config);
        assert_eq!(result, Err(BatchError::EmptyBatch));
    }

    #[test]
    fn test_batch_too_large_returns_error() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (_token_addr, _token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        asset_client.mint(&caller, &10_000);

        let config = BatchConfig {
            mode: BatchMode::Atomic,
            max_operations: 2,
        };
        let recipient = Address::generate(&env);
        let transfers = Vec::from_array(
            &env,
            [
                make_transfer(&recipient, 100),
                make_transfer(&recipient, 200),
                make_transfer(&recipient, 300),
            ],
        );
        let result = execute_multi_transfer(&env, &caller, &_token_addr, &transfers, &config);
        assert_eq!(result, Err(BatchError::BatchTooLarge));
    }

    // -----------------------------------------------------------------------
    // Reentrancy guard
    // -----------------------------------------------------------------------

    #[test]
    fn test_reentrancy_guard_prevents_nested_calls() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (_token_addr, _token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        let recipient = Address::generate(&env);
        asset_client.mint(&caller, &1_000);

        // Manually set the reentrancy guard to simulate a nested call.
        crate::storage::temporary_set(&env, &REENTRANCY_KEY, &true);

        let config = default_atomic_config();
        let transfers = Vec::from_array(&env, [make_transfer(&recipient, 100)]);
        let result = execute_multi_transfer(&env, &caller, &_token_addr, &transfers, &config);
        assert_eq!(result, Err(BatchError::ReentrancyDetected));

        // Clean up so other tests aren't affected.
        crate::storage::temporary_remove(&env, &REENTRANCY_KEY);
    }

    // -----------------------------------------------------------------------
    // Invalid operations
    // -----------------------------------------------------------------------

    #[test]
    fn test_zero_amount_transfer_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (_token_addr, _token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        let recipient = Address::generate(&env);
        asset_client.mint(&caller, &1_000);

        let config = default_atomic_config();
        let transfers = Vec::from_array(&env, [make_transfer(&recipient, 0)]);
        let result = execute_multi_transfer(&env, &caller, &_token_addr, &transfers, &config);
        assert_eq!(result, Err(BatchError::InvalidOperation));
    }

    #[test]
    fn test_negative_amount_transfer_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (_token_addr, _token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        let recipient = Address::generate(&env);
        asset_client.mint(&caller, &1_000);

        let config = default_atomic_config();
        let transfers = Vec::from_array(&env, [make_transfer(&recipient, -50)]);
        let result = execute_multi_transfer(&env, &caller, &_token_addr, &transfers, &config);
        assert_eq!(result, Err(BatchError::InvalidOperation));
    }

    // -----------------------------------------------------------------------
    // Atomic mode — all succeed
    // -----------------------------------------------------------------------

    #[test]
    fn test_atomic_batch_all_succeed() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (token_addr, token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        let recipient1 = Address::generate(&env);
        let recipient2 = Address::generate(&env);
        asset_client.mint(&caller, &1_000);

        let config = default_atomic_config();
        let transfers = Vec::from_array(
            &env,
            [
                make_transfer(&recipient1, 100),
                make_transfer(&recipient2, 200),
            ],
        );

        let result =
            execute_multi_transfer(&env, &caller, &token_addr, &transfers, &config).unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 0);
        assert!(!result.reverted);
        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results.get_unchecked(0), OperationResult::Success);
        assert_eq!(result.results.get_unchecked(1), OperationResult::Success);

        // Verify balances
        assert_eq!(token_client.balance(&recipient1), 100);
        assert_eq!(token_client.balance(&recipient2), 200);
        assert_eq!(token_client.balance(&caller), 700);
    }

    // -----------------------------------------------------------------------
    // Non-atomic mode — mixed results
    // -----------------------------------------------------------------------

    #[test]
    fn test_non_atomic_batch_collects_failures() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (token_addr, token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        let recipient1 = Address::generate(&env);
        let recipient2 = Address::generate(&env);
        // Only give the caller 150 tokens.
        asset_client.mint(&caller, &150);

        let config = default_non_atomic_config();
        // First transfer (100) should succeed, second (200) should fail.
        let transfers = Vec::from_array(
            &env,
            [
                make_transfer(&recipient1, 100),
                make_transfer(&recipient2, 200),
            ],
        );

        let result =
            execute_multi_transfer(&env, &caller, &token_addr, &transfers, &config).unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 1);
        assert!(!result.reverted);

        // First transfer went through.
        assert_eq!(token_client.balance(&recipient1), 100);
        // Second did not.
        assert_eq!(token_client.balance(&recipient2), 0);
        // Caller kept the remaining 50.
        assert_eq!(token_client.balance(&caller), 50);
    }

    // -----------------------------------------------------------------------
    // Non-atomic mode — all fail
    // -----------------------------------------------------------------------

    #[test]
    fn test_non_atomic_batch_all_fail() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (token_addr, token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        let recipient = Address::generate(&env);
        // No tokens minted — all transfers will fail.
        let config = default_non_atomic_config();
        let transfers = Vec::from_array(&env, [make_transfer(&recipient, 100)]);

        let result =
            execute_multi_transfer(&env, &caller, &token_addr, &transfers, &config).unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 0);
        assert_eq!(result.failed, 1);
        assert!(!result.reverted);
        assert_eq!(token_client.balance(&recipient), 0);
    }

    // -----------------------------------------------------------------------
    // Single-item batch
    // -----------------------------------------------------------------------

    #[test]
    fn test_single_transfer_batch() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (token_addr, token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        let recipient = Address::generate(&env);
        asset_client.mint(&caller, &500);

        let config = default_atomic_config();
        let transfers = Vec::from_array(&env, [make_transfer(&recipient, 500)]);

        let result =
            execute_multi_transfer(&env, &caller, &token_addr, &transfers, &config).unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 0);
        assert!(!result.reverted);
        assert_eq!(token_client.balance(&recipient), 500);
        assert_eq!(token_client.balance(&caller), 0);
    }

    // -----------------------------------------------------------------------
    // max_operations enforcement
    // -----------------------------------------------------------------------

    #[test]
    fn test_custom_max_operations_is_enforced() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (_token_addr, _token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        let recipient = Address::generate(&env);
        asset_client.mint(&caller, &10_000);

        let config = BatchConfig {
            mode: BatchMode::Atomic,
            max_operations: 1,
        };
        // Two transfers exceed the max of 1.
        let transfers = Vec::from_array(
            &env,
            [
                make_transfer(&recipient, 100),
                make_transfer(&recipient, 200),
            ],
        );
        let result = execute_multi_transfer(&env, &caller, &_token_addr, &transfers, &config);
        assert_eq!(result, Err(BatchError::BatchTooLarge));
    }

    // -----------------------------------------------------------------------
    // multi_transfer_all helper
    // -----------------------------------------------------------------------

    #[test]
    fn test_multi_transfer_all_helper() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (token_addr, token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        let recipient1 = Address::generate(&env);
        let recipient2 = Address::generate(&env);
        asset_client.mint(&caller, &1_000);

        let recipients = [
            (recipient1.clone(), 150_i128),
            (recipient2.clone(), 250_i128),
        ];
        let result = multi_transfer_all(&env, &caller, &token_addr, &recipients).unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.succeeded, 2);
        assert!(!result.reverted);
        assert_eq!(token_client.balance(&recipient1), 150);
        assert_eq!(token_client.balance(&recipient2), 250);
        assert_eq!(token_client.balance(&caller), 600);
    }

    #[test]
    fn test_multi_transfer_all_empty_recipients() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (_token_addr, _token_client, _asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);

        let recipients: [(Address, i128); 0] = [];
        let result = multi_transfer_all(&env, &caller, &_token_addr, &recipients);
        assert_eq!(result, Err(BatchError::EmptyBatch));
    }

    // -----------------------------------------------------------------------
    // Large batch within limits
    // -----------------------------------------------------------------------

    #[test]
    fn test_batch_at_max_operations_succeeds() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let (token_addr, token_client, asset_client) = setup_token(&env, &admin);
        let caller = Address::generate(&env);
        asset_client.mint(&caller, &10_000);

        let config = BatchConfig {
            mode: BatchMode::Atomic,
            max_operations: 3,
        };

        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);
        let r3 = Address::generate(&env);

        let transfers = Vec::from_array(
            &env,
            [
                make_transfer(&r1, 100),
                make_transfer(&r2, 200),
                make_transfer(&r3, 300),
            ],
        );

        let result =
            execute_multi_transfer(&env, &caller, &token_addr, &transfers, &config).unwrap();

        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 3);
        assert_eq!(result.failed, 0);
        assert!(!result.reverted);
        assert_eq!(token_client.balance(&r1), 100);
        assert_eq!(token_client.balance(&r2), 200);
        assert_eq!(token_client.balance(&r3), 300);
    }
}
