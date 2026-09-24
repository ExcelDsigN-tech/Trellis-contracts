use soroban_sdk::{symbol_short, Address, Env, Symbol};

/// Emit when a submitter is registered.
pub fn emit_submitter_registered(env: &Env, submitter: &Address, timestamp: u64) {
    env.events().publish(
        (symbol_short!("oracle"), symbol_short!("sub_reg")),
        (submitter.clone(), timestamp),
    );
}

/// Emit when a submitter is deactivated.
pub fn emit_submitter_deactivated(env: &Env, submitter: &Address, timestamp: u64) {
    env.events().publish(
        (symbol_short!("oracle"), symbol_short!("sub_del")),
        (submitter.clone(), timestamp),
    );
}

/// Emit when a price is submitted.
pub fn emit_price_submitted(
    env: &Env,
    feed_id: &Symbol,
    price: i128,
    submitter: &Address,
    timestamp: u64,
) {
    env.events().publish(
        (symbol_short!("oracle"), symbol_short!("price")),
        (feed_id.clone(), price, submitter.clone(), timestamp),
    );
}

/// Emit when a staleness window is updated.
pub fn emit_staleness_window_set(env: &Env, window_seconds: u64, timestamp: u64) {
    env.events().publish(
        (symbol_short!("oracle"), symbol_short!("stale")),
        (window_seconds, timestamp),
    );
}
