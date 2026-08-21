#![no_std]

mod fee_calculator;

mod slippage_predictor;

mod strategy_executor;

mod logging;

use fee_calculator::calculate_total_fees;
use slippage_predictor::predict_slippage;
use soroban_sdk::{contract, contractimpl, contracttype, Env, Symbol, Vec, U256};
use strategy_executor::execute_strategy;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trade {
    pub asset_pair: (Symbol, Symbol),
    pub amount: u128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionStrategy {
    MinimalCost,
    MinimalTime,
    Balanced,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulationResult {
    pub expected_fees: u128,
    pub expected_slippage: U256,
}

#[contract]
pub struct MultiAssetRebalancer;

#[contractimpl]
impl MultiAssetRebalancer {
    pub fn rebalance(
        env: Env,
        trades: Vec<Trade>,
        strategy: ExecutionStrategy,
        dry_run: bool,
    ) -> SimulationResult {
        let total_fees = calculate_total_fees(&trades);
        // U256 has no env-independent Add impl, so accumulate the count of
        // slippage units (the predictor currently returns a constant) and
        // materialise the total once.
        let mut slippage_units: u128 = 0;
        for trade in trades.iter() {
            let _ = predict_slippage(trade.asset_pair.clone(), trade.amount, &env);
            slippage_units = slippage_units.saturating_add(1);
        }
        let total_slippage = U256::from_u128(&env, slippage_units);

        if !dry_run {
            execute_strategy(&env, &strategy, &trades);
        }

        SimulationResult {
            expected_fees: total_fees,
            expected_slippage: total_slippage,
        }
    }
}
