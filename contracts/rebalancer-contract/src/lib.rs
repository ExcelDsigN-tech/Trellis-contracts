#![no_std]

mod fee_calculator;

mod slippage_predictor;

mod strategy_executor;

mod logging;

use soroban_sdk::{contract, contractimpl, Env, Symbol, Vec};
use soroban_sdk::U256;
use fee_calculator::calculate_total_fees;
use slippage_predictor::predict_slippage;
use strategy_executor::execute_strategy;
use logging::log_trade;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trade {
    pub asset_pair: (Symbol, Symbol),
    pub amount: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionStrategy {
    MinimalCost,
    MinimalTime,
    Balanced,
}

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
        let mut total_slippage = U256::from_u32(&env, 0);
        for trade in trades.iter() {
            total_slippage = total_slippage + predict_slippage(trade.asset_pair.clone(), trade.amount, &env);
        }

        if !dry_run {
            execute_strategy(&env, &strategy, &trades);
        }

        SimulationResult {
            expected_fees: total_fees,
            expected_slippage: total_slippage,
        }
    }
}