use soroban_sdk::{
    contract, contractimpl, contracterror, panic_with_error, token, symbol_short, Address, Env, Symbol, Map,
};
use shared::events::{emit_aid_created, emit_action_executed, emit_module_initialized, emit_permission_changed};
use shared::{emit, AID_CLAIMED, AID_CREATED, AID_REFUNDED, AID_SETTLED, Error};
use shared::storage::is_paused;

const KEY_AIDS: Symbol = symbol_short!("aids");

pub mod storage;
pub mod types;

use storage::{get_aid, has_aid, set_aid};

// Re-export so test modules (and `use super::*`) have access.
pub use types::{AidRecord, AidStatus};

// ---------------------------------------------------------------------------
// Contract-specific error codes  (range 100-199 per shared/README.md)
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AidError {
    /// Caller is not the authorised recipient.
    Unauthorized = 100,
    /// The requested aid record was not found.
    NotFound = 101,
    /// The aid has already been settled or refunded.
    AlreadyClaimed = 102,
    /// The claim window has expired (past `expiry_ledger`).
    Expired = 103,
    /// The contract is paused; no state-changing operations are allowed.
    Paused = 104,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct AidContract;

#[contractimpl]
impl AidContract {
    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    /// Initialise the contract, storing the admin address.
    ///
    /// Must be called exactly once immediately after deployment.
    pub fn initialize(env: Env, admin: Address) {
        shared::auth::set_admin(&env, &admin);
        emit_module_initialized(&env, symbol_short!("aid"), 1, &admin, env.ledger().timestamp());
    }

    // -----------------------------------------------------------------------
    // Aid creation
    // -----------------------------------------------------------------------

    /// Create a new aid disbursement and escrow funds from the donor.
    ///
    /// Transfers `amount` of `token` from `donor` to this contract for
    /// safekeeping until the recipient claims or the aid expires.
    ///
    /// Returns the newly allocated `aid_id`.
    pub fn create_aid(
        env: Env,
        aid_id: u64,
        donor: Address,
        recipient: Address,
        token: Address,
        amount: i128,
        expiry_ledger: u32,
    ) -> u64 {
        donor.require_auth();

        if amount <= 0 {
            env.panic_with_error(shared::Error::InvalidAmount);
        }
        if expiry_ledger <= env.ledger().sequence() {
            env.panic_with_error(shared::Error::InvalidArgument);
        }
        if has_aid(&env, aid_id) {
            env.panic_with_error(shared::Error::InvalidArgument);
        }

        // Escrow funds from donor into contract.
        token::Client::new(&env, &token).transfer(
            &donor,
            &env.current_contract_address(),
            &amount,
        );

        let record = AidRecord {
            id: aid_id,
            donor: donor.clone(),
            recipient: recipient.clone(),
            token: token.clone(),
            amount,
            expiry_ledger,
            status: AidStatus::Pending,
        };
        set_aid(&env, aid_id, &record);

        let mut aids: Map<u64, AidRecord> = env.storage()
            .persistent()
            .get(&KEY_AIDS)
            .unwrap_or_else(|| Map::new(&env));
        aids.set(aid_id, record);
        env.storage().persistent().set(&KEY_AIDS, &aids);

        emit_aid_created(
            &env,
            aid_id,
            &donor,
            &recipient,
            amount,
            env.ledger().sequence().into(),
            expiry_ledger.into(),
        );

        emit(&env, AID_CREATED, (aid_id, donor, recipient, amount, expiry_ledger));
        emit_action_executed(&env, symbol_short!("aid"), symbol_short!("create"), &env.current_contract_address(), true, env.ledger().timestamp());
        aid_id
    }

    // -----------------------------------------------------------------------
    // Aid claiming
    // -----------------------------------------------------------------------

    /// Claim a pending aid disbursement and transfer funds to the recipient.
    ///
    /// # Errors (via `env.panic_with_error`)
    /// - [`AidError::Paused`]         — contract is paused.
    /// - [`AidError::NotFound`]       — `aid_id` does not exist.
    /// - [`AidError::Expired`]        — `expiry_ledger` has passed.
    /// - [`AidError::AlreadyClaimed`] — status is not `Pending`.
    /// - [`AidError::Unauthorized`]   — `caller` is not the recipient.
    pub fn claim_aid(env: Env, aid_id: u64, caller: Address) -> Result<(), AidError> {
        if is_paused(&env) {
            return Err(AidError::Paused);
        }
        caller.require_auth();

        let mut record = get_aid(&env, aid_id).ok_or(AidError::NotFound)?;

        if env.ledger().sequence() > record.expiry_ledger {
            return Err(AidError::Expired);
        }
        if record.status != AidStatus::Pending {
            return Err(AidError::AlreadyClaimed);
        }
        if caller != record.recipient {
            return Err(AidError::Unauthorized);
        }

        // Checks-effects-interactions: write status first, then transfer.
        record.status = AidStatus::Settled;
        set_aid(&env, aid_id, &record);

        token::Client::new(&env, &record.token).transfer(
            &env.current_contract_address(),
            &record.recipient,
            &record.amount,
        );

        emit(&env, AID_CLAIMED, aid_id);
        emit(&env, AID_SETTLED, aid_id);
        emit_action_executed(&env, symbol_short!("aid"), symbol_short!("claim_aid"), &env.current_contract_address(), true, env.ledger().timestamp());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Refunds
    // -----------------------------------------------------------------------

    /// Refund an expired, unclaimed aid disbursement to the original donor.
    ///
    /// Anyone may call this after expiry to trigger a refund; it is not
    /// gated to the admin so expired funds cannot be held hostage.
    ///
    /// # Errors (via `env.panic_with_error`)
    /// - [`AidError::NotFound`]       — `aid_id` does not exist.
    /// - [`AidError::AlreadyClaimed`] — already settled or refunded.
    /// - [`shared::Error::InvalidArgument`] — expiry has not yet passed.
    pub fn refund_expired(env: Env, aid_id: u64) -> Result<(), AidError> {
        let mut record = get_aid(&env, aid_id).ok_or(AidError::NotFound)?;

        if record.status != AidStatus::Pending {
            return Err(AidError::AlreadyClaimed);
        }
        if env.ledger().sequence() <= record.expiry_ledger {
            env.panic_with_error(shared::Error::InvalidArgument);
        }

        // Checks-effects-interactions.
        record.status = AidStatus::Refunded;
        set_aid(&env, aid_id, &record);

        token::Client::new(&env, &record.token).transfer(
            &env.current_contract_address(),
            &record.donor,
            &record.amount,
        );

        emit(&env, AID_REFUNDED, aid_id);
        emit_action_executed(&env, symbol_short!("aid"), symbol_short!("refund"), &env.current_contract_address(), true, env.ledger().timestamp());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Return the aid record for `aid_id`, or `None` if it does not exist.
    pub fn get_aid(env: Env, aid_id: u64) -> Option<AidRecord> {
        storage::get_aid(&env, aid_id)
    }

    /// Set the paused state of the contract.
    pub fn set_paused(env: Env, admin: Address, paused: bool) {
        let contract_admin = shared::auth::get_admin(&env);
        if admin != contract_admin {
             panic_with_error!(env, Error::Unauthorized);
        }
        admin.require_auth();

        env.storage().instance().set(&Symbol::new(&env, "paused"), &paused);
        emit_permission_changed(&env, symbol_short!("aid"), symbol_short!("paused"), &admin, paused, env.ledger().timestamp());
    }
}

#[cfg(test)]
mod tests;
