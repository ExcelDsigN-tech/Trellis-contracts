
use soroban_sdk::{Env, Symbol, IntoVal, Val};

pub const AID_CREATED: Symbol = Symbol::new("aid_created");
pub const AID_CLAIMED: Symbol = Symbol::new("aid_claimed");
pub const AID_SETTLED: Symbol = Symbol::new("aid_settled");
pub const AID_REFUNDED: Symbol = Symbol::new("aid_refunded");

pub const CONTRACT_UPGRADED: Symbol = Symbol::new("contract_upgraded");
pub const CONTRACT_PAUSED: Symbol = Symbol::new("contract_paused");
pub const CONTRACT_RESUMED: Symbol = Symbol::new("contract_resumed");

pub const PARAMETER_CHANGED: Symbol = Symbol::new("parameter_changed");
pub const COMMISSION_PAID: Symbol = Symbol::new("commission_paid");

pub fn emit<T: IntoVal<Env, Val>>(env: &Env, topic: Symbol, data: T) {
    env.events().publish((topic,), data);
}