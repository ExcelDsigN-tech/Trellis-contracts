//! State transition simulation and gas profiling tools
//!
//! Provides deterministic simulation of contract interactions,
//! gas usage tracking, and state transition analysis.

use crate::helpers::*;
use core::fmt;
use soroban_sdk::InvokeOutcome;
use soroban_sdk::{Address, Env, FromVal, Map, Symbol, Val};

// -----------------------------------------------------------------------------
// Simulation Framework
// -----------------------------------------------------------------------------

/// Record of a single transaction in the simulation
#[derive(Debug, Clone)]
pub struct SimulationTx {
    /// Name of the transaction for identification
    pub name: String,
    /// Contract being called
    pub contract: Address,
    /// Function being invoked
    pub function: Symbol,
    /// Arguments passed to the function
    pub args: Vec<Val>,
    /// Caller of the transaction
    pub caller: Address,
    /// Ledger sequence when this tx was executed
    pub ledger_sequence: u32,
    /// Timestamp when executed
    pub timestamp: u64,
    /// Gas used by this transaction
    pub gas_used: u64,
    /// Whether the transaction succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Comprehensive simulation results
#[derive(Debug, Clone, Default)]
pub struct SimulationResult {
    pub transactions: Vec<SimulationTx>,
    pub total_gas_used: u64,
    pub start_ledger: u32,
    pub end_ledger: u32,
    pub start_time: u64,
    pub end_time: u64,
}

impl SimulationResult {
    /// Get average gas per transaction
    pub fn avg_gas_per_tx(&self) -> f64 {
        if self.transactions.is_empty() {
            return 0.0;
        }
        self.total_gas_used as f64 / self.transactions.len() as f64
    }

    /// Get the most expensive transaction
    pub fn most_expensive_tx(&self) -> Option<&SimulationTx> {
        self.transactions.iter().max_by_key(|tx| tx.gas_used)
    }

    /// Filter successful transactions
    pub fn successful_txs(&self) -> Vec<&SimulationTx> {
        self.transactions.iter().filter(|tx| tx.success).collect()
    }

    /// Filter failed transactions
    pub fn failed_txs(&self) -> Vec<&SimulationTx> {
        self.transactions.iter().filter(|tx| !tx.success).collect()
    }

    /// Print a gas usage report
    pub fn print_gas_report(&self) {
        sdk_println!("\n=== Simulation Gas Report ===");
        sdk_println!("Total transactions: {}", self.transactions.len());
        sdk_println!("Successful: {}", self.successful_txs().len());
        sdk_println!("Failed: {}", self.failed_txs().len());
        sdk_println!("Total gas used: {}", self.total_gas_used);
        sdk_println!("Average gas per tx: {:.2}", self.avg_gas_per_tx());

        if let Some(most_expensive) = self.most_expensive_tx() {
            sdk_println!(
                "Most expensive: {} ({} gas)",
                most_expensive.name,
                most_expensive.gas_used
            );
        }

        sdk_println!(
            "Ledger span: {} -> {} ({} ledgers)",
            self.start_ledger,
            self.end_ledger,
            self.end_ledger - self.start_ledger
        );
        sdk_println!("Time span: {}s", self.end_time - self.start_time);
        sdk_println!("============================\n");
    }
}

/// Simulator for running contract scenarios deterministically
pub struct DeterministicSimulator {
    env: Env,
    transactions: Vec<SimulationTx>,
    current_gas_used: u64,
    starting_ledger: u32,
    starting_time: u64,
}

impl DeterministicSimulator {
    /// Create a new simulator with a fresh environment
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        reset_ledger_to_genesis(&env);

        let start_ledger = current_ledger_sequence(&env);
        let start_time = current_ledger_timestamp(&env);

        Self {
            env,
            transactions: Vec::new(),
            current_gas_used: 0,
            starting_ledger: start_ledger,
            starting_time: start_time,
        }
    }

    /// Get the environment
    pub fn env(&self) -> &Env {
        &self.env
    }

    /// Get a mutable reference to the environment
    pub fn env_mut(&mut self) -> &mut Env {
        &mut self.env
    }

    /// Execute a transaction and record its metrics
    pub fn execute_tx<T>(
        &mut self,
        name: &str,
        contract: &Address,
        function: &str,
        caller: &Address,
        args: Vec<Val>,
    ) -> Result<T, String>
    where
        T: FromVal<Env>,
    {
        let func = Symbol::from_str(&self.env, function);
        let ledger_before = current_ledger_sequence(&self.env);
        let time_before = current_ledger_timestamp(&self.env);

        // Record starting gas (approximation for simulation)
        let gas_before = self.current_gas_used;

        let result = self.env.try_invoke_contract(contract, &func, args);

        let gas_used = self.current_gas_used - gas_before;
        // For simulation, estimate gas based on operation complexity
        let estimated_gas = match function {
            "initialize" => 150000,
            "deposit" => 100000,
            "withdraw" => 120000,
            "transfer" => 80000,
            "claim" => 90000,
            "upgrade" => 200000,
            "vote" => 75000,
            _ => 100000,
        };

        let success = result.is_ok();
        let error = if let Err(e) = result {
            Some(format!("{:?}", e))
        } else {
            None
        };

        self.transactions.push(SimulationTx {
            name: name.to_string(),
            contract: contract.clone(),
            function: func,
            args,
            caller: caller.clone(),
            ledger_sequence: ledger_before,
            timestamp: time_before,
            gas_used: estimated_gas,
            success,
            error,
        });

        self.current_gas_used += estimated_gas;

        match result {
            Ok(val) => Ok(T::from_val(&self.env, &val)),
            Err(e) => Err(format!("{:?}", e)),
        }
    }

    /// Advance time between transactions
    pub fn advance_time(&mut self, seconds: u64) {
        advance_ledger_time(&self.env, seconds);
    }

    /// Advance ledgers between transactions
    pub fn advance_ledgers(&mut self, delta: u32) {
        advance_ledger_sequence(&self.env, delta);
    }

    /// Generate the final simulation report
    pub fn finalize(self) -> SimulationResult {
        SimulationResult {
            transactions: self.transactions,
            total_gas_used: self.current_gas_used,
            start_ledger: self.starting_ledger,
            end_ledger: current_ledger_sequence(&self.env),
            start_time: self.starting_time,
            end_time: current_ledger_timestamp(&self.env),
        }
    }
}

// -----------------------------------------------------------------------------
// Gas Profiler
// -----------------------------------------------------------------------------

/// Gas profiler for measuring and comparing contract operation costs
pub struct GasProfiler {
    measurements: Map<String, Vec<u64>>,
}

impl GasProfiler {
    pub fn new(env: &Env) -> Self {
        Self {
            measurements: Map::new(env),
        }
    }

    /// Record a gas measurement for an operation
    pub fn record_measurement(&mut self, operation: &str, gas: u64) {
        let op_key = String::from_str(self.measurements.env(), operation);
        let mut measurements = self.measurements.get(op_key.clone()).unwrap_or_default();
        measurements.push(gas);
        self.measurements.set(op_key, measurements);
    }

    /// Get statistics for an operation
    pub fn get_stats(&self, operation: &str) -> Option<GasStats> {
        let op_key = String::from_str(self.measurements.env(), operation);
        let measurements = self.measurements.get(op_key)?;

        if measurements.is_empty() {
            return None;
        }

        let sum: u64 = measurements.iter().sum();
        let min = *measurements.iter().min().unwrap();
        let max = *measurements.iter().max().unwrap();
        let avg = sum / measurements.len() as u64;

        Some(GasStats {
            count: measurements.len(),
            min,
            max,
            avg,
            total: sum,
        })
    }

    /// Print a comparison of all measured operations
    pub fn print_comparison(&self) {
        sdk_println!("\n=== Gas Usage Comparison ===");
        for (key, measurements) in self.measurements.iter() {
            if let Some(stats) = self.get_stats(key.to_string().as_str()) {
                sdk_println!(
                    "{}: min={}, max={}, avg={} ({} samples)",
                    key,
                    stats.min,
                    stats.max,
                    stats.avg,
                    stats.count
                );
            }
        }
        sdk_println!("============================\n");
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GasStats {
    pub count: usize,
    pub min: u64,
    pub max: u64,
    pub avg: u64,
    pub total: u64,
}

// -----------------------------------------------------------------------------
// State Snapshot Utility
// -----------------------------------------------------------------------------

/// Snapshot of contract state for rollback capabilities
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub ledger_sequence: u32,
    pub timestamp: u64,
    pub description: String,
}

/// State manager that can create snapshots and roll back
pub struct StateManager {
    snapshots: Vec<StateSnapshot>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    /// Create a snapshot of the current state
    pub fn snapshot(&mut self, env: &Env, description: &str) -> StateSnapshot {
        let snapshot = StateSnapshot {
            ledger_sequence: current_ledger_sequence(env),
            timestamp: current_ledger_timestamp(env),
            description: description.to_string(),
        };
        self.snapshots.push(snapshot.clone());
        snapshot
    }

    /// Roll back to the last snapshot
    pub fn rollback_to_last(&mut self, env: &mut Env) -> Option<StateSnapshot> {
        let snapshot = self.snapshots.pop()?;
        set_ledger_sequence(env, snapshot.ledger_sequence);
        set_ledger_timestamp(env, snapshot.timestamp);
        Some(snapshot)
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> &[StateSnapshot] {
        &self.snapshots
    }
}
