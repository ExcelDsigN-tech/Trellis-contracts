use soroban_sdk::{symbol_short, Env};

use crate::Trade;
use shared::events::emit_action_executed;

pub fn log_trade(env: &Env, _trade: &Trade, _actual_price: u128, _fee: u128) {
    emit_action_executed(
        env,
        symbol_short!("reb"),
        symbol_short!("trade"),
        &env.current_contract_address(),
        true,
        env.ledger().timestamp(),
    );
}