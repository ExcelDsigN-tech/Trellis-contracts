#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

use shared::events::emit_module_initialized;

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    /// Initialise the contract, setting the admin address.
    pub fn initialize(env: Env, admin: Address) {
        shared::auth::set_admin(&env, &admin);
        emit_module_initialized(
            &env,
            symbol_short!("oracle"),
            1,
            &admin,
            env.ledger().timestamp(),
        );
    }
}
