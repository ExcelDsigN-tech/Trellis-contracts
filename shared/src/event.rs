use soroban_sdk::{symbol_short, Env, IntoVal, Symbol, Val};

pub const AID_CREATED: Symbol = symbol_short!("aid_crt");
pub const AID_CLAIMED: Symbol = symbol_short!("aid_clm");
pub const AID_SETTLED: Symbol = symbol_short!("aid_stl");
pub const AID_REFUNDED: Symbol = symbol_short!("aid_ref");

pub const CONTRACT_UPGRADED: Symbol = symbol_short!("upgraded");
pub const CONTRACT_PAUSED: Symbol = symbol_short!("paused");
pub const CONTRACT_RESUMED: Symbol = symbol_short!("resumed");

pub const PARAMETER_CHANGED: Symbol = symbol_short!("param_chg");
pub const COMMISSION_PAID: Symbol = symbol_short!("com_paid");

pub fn emit<T: IntoVal<Env, Val>>(env: &Env, topic: Symbol, data: T) {
    env.events().publish((topic,), data);
}
