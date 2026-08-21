#![no_std]

mod fee_calculator;

mod slippage_predictor;

mod strategy_executor;

mod logging;

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Env, Symbol, Vec};
use soroban_sdk::U256;
use fee_calculator::calculate_total_fees;
use logging::log_trade;
use slippage_predictor::predict_slippage;
use soroban_sdk::U256;
use soroban_sdk::{contract, contractimpl, Env, Symbol, Vec};
use strategy_executor::execute_strategy;
use shared::events::emit_action_executed;

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
        let mut total_slippage = U256::from_u32(&env, 0);
        for trade in trades.iter() {
            total_slippage = total_slippage.add(&predict_slippage(trade.asset_pair.clone(), trade.amount, &env));
        }

        if !dry_run {
            execute_strategy(&env, &strategy, &trades);
        }

        let result = SimulationResult {
            expected_fees: total_fees,
            expected_slippage: total_slippage,
        };

        emit_action_executed(
            &env,
            symbol_short!("reb"),
            symbol_short!("rebal"),
            &env.current_contract_address(),
            !dry_run,
            env.ledger().timestamp(),
        );

        result
    }
}
