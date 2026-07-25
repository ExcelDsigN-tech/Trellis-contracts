#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Env,
};

#[test]
fn test_rebalance_dry_run() {
    let env = Env::default();
    let contract_id = env.register_contract(None, MultiAssetRebalancer);
    let client = MultiAssetRebalancerClient::new(&env, &contract_id);

    let mut trades = Vec::new(&env);
    trades.push(Trade {
        asset_pair: (Symbol::new(&env, "USDC"), Symbol::new(&env, "XLM")),
        amount: 1000,
    });

    let result = client.rebalance(&trades, &ExecutionStrategy::Balanced, &true);

    assert_eq!(result.expected_fees, 1);
    assert_eq!(result.expected_slippage, U256::from_u32(&env, 1));
}