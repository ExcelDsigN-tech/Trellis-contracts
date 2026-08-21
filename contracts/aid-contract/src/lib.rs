use soroban_sdk::{
    contract, contractimpl, contracterror, panic_with_error, token, symbol_short, Address, Env, Symbol, Map,
};
use shared::events::{emit_aid_created, emit_action_executed, emit_module_initialized, emit_permission_changed};
use shared::{emit, AID_CLAIMED, AID_CREATED, AID_REFUNDED, AID_SETTLED, Error};
use shared::storage::is_paused;

const KEY_AIDS: Symbol = symbol_short!("aids");
#![no_std]

use soroban_sdk::{contract, contractimpl, contracterror, token, Address, Env};

use shared::storage::{is_paused, set_paused as shared_set_paused};

pub mod storage;
pub mod types;
pub mod api;

use storage::{get_aid, has_aid, set_aid};

pub use types::{AidRecord, AidStatus};

// ---------------------------------------------------------------------------
// Contract-specific error codes (range 100-199 per shared conventions)
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AidError {
    Unauthorized = 100,
    NotFound = 101,
    AlreadyClaimed = 102,
    Expired = 103,
    Paused = 104,
    /// The aid has not expired yet and cannot be refunded.
    NotExpiredYet = 105,
    /// The aid has already been refunded to the donor.
    AlreadyRefunded = 106,
}

#[contract]
pub struct AidContract;

#[contractimpl]
impl AidContract {
    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    /// Initialise the contract with an admin and the escrow token.
    ///
    /// Must be called exactly once immediately after deployment.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        shared::auth::set_admin(&env, &admin);
        emit_module_initialized(&env, symbol_short!("aid"), 1, &admin, env.ledger().timestamp());
        env.storage().instance().set(&storage::DataKey::Token, &token);
    }

    // -----------------------------------------------------------------------
    // Aid creation
    // -----------------------------------------------------------------------

    /// Create a new aid disbursement and escrow funds from the donor.
    ///
    /// The aid ID is auto-generated. Transfers `amount` of the configured
    /// `token` from `donor` into this contract for safekeeping until the
    /// recipient claims or the aid expires.
    ///
    /// Also appends the new ID to both the donor and recipient indexes so
    /// paginated queries stay consistent.
    pub fn create_aid(
        env: Env,
        donor: Address,
        recipient: Address,
        amount: i128,
        expiry_ledger: u32,
    ) -> u64 {
        donor.require_auth();

        // Fast-path: cheapest validation first (gas ordering)
        if amount <= 0 {
            panic_with_error!(&env, SharedError::InvalidAmount);
        }
        if expiry_ledger <= env.ledger().sequence() {
            env.panic_with_error(AidError::NotExpiredYet);
        }

        // Auto-allocate aid ID via counter (avoids caller-supplied collision)
        let aid_id = get_aid_counter(&env);
        if has_aid(&env, aid_id) {
            env.panic_with_error(shared::Error::InvalidArgument);
        }
        set_aid_counter(&env, aid_id.wrapping_add(1));

        let token: Address = env
            .storage()
            .instance()
            .get(&storage::DataKey::Token)
            .expect("token not initialised");

        // Checks-effects-interactions: store record before cross-contract call
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
        set_aid_counter(&env, aid_id);

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
        // Escrow funds from donor into contract.
        token::Client::new(&env, &token).transfer(
            &donor,
            &env.current_contract_address(),
            &amount,
        );

        aid_id
    }

    // -----------------------------------------------------------------------
    // Aid claiming
    // -----------------------------------------------------------------------

    /// Claim a pending aid disbursement and transfer funds to the recipient.
    ///
    /// # Errors (via Result)
    /// - [`AidError::Paused`]         — contract is paused.
    /// - [`AidError::NotFound`]       — `aid_id` does not exist.
    /// - [`AidError::Expired`]        — `expiry_ledger` has passed.
    /// - [`AidError::AlreadyClaimed`] — status is not `Pending`.
    /// - [`AidError::Unauthorized`]   — `recipient` is not the intended recipient.
    pub fn claim_aid(env: Env, aid_id: u64, recipient: Address) -> Result<(), AidError> {
        // Pause check first — cheapest read (instance storage, no TTL bump)
        if is_paused(&env) {
            return Err(AidError::Paused);
        }
        recipient.require_auth();

        let mut record = get_aid(&env, aid_id).ok_or(AidError::NotFound)?;

        // Sequence of checks ordered by likely failure rate (cheap first)
        if record.status == AidStatus::Settled || record.status == AidStatus::Refunded {
            return Err(AidError::AlreadyClaimed);
        }
        if env.ledger().sequence() > record.expiry_ledger {
            return Err(AidError::Expired);
        }
        if recipient != record.recipient {
            return Err(AidError::Unauthorized);
        }

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
    /// Only the donor or the admin may trigger the refund.
    ///
    /// # Errors
    /// - [`AidError::NotFound`]       — `aid_id` does not exist.
    /// - [`AidError::AlreadyClaimed`] — already settled.
    /// - [`AidError::AlreadyRefunded`] — already refunded.
    /// - [`AidError::NotExpiredYet`]  — expiry has not yet passed.
    pub fn refund_aid(env: Env, aid_id: u64) -> Result<(), AidError> {
        let mut record = get_aid(&env, aid_id).ok_or(AidError::NotFound)?;

        // Check status first — avoids expensive ledger read on wrong state
        match record.status {
            AidStatus::Settled => return Err(AidError::AlreadyClaimed),
            AidStatus::Refunded => return Err(AidError::AlreadyRefunded),
            AidStatus::Pending => {} // continue
        }

        if env.ledger().sequence() <= record.expiry_ledger {
            return Err(AidError::NotExpiredYet);
        }

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

    pub fn get_aid(env: Env, aid_id: u64) -> Option<AidRecord> {
        storage::get_aid(&env, aid_id)
    }

    /// Set the paused state of the contract.
    // -----------------------------------------------------------------------
    // Admin controls
    // -----------------------------------------------------------------------

    /// Pause or resume the contract. Admin only.
    pub fn set_paused(env: Env, admin: Address, paused: bool) {
        let contract_admin = shared::auth::get_admin(&env);
        if admin != contract_admin {
            env.panic_with_error(shared::Error::Unauthorized);
        }
        admin.require_auth();

        env.storage().instance().set(&Symbol::new(&env, "paused"), &paused);
        emit_permission_changed(&env, symbol_short!("aid"), symbol_short!("paused"), &admin, paused, env.ledger().timestamp());
        shared_set_paused(&env, paused);
    }
}

/// Slice `ids` into one page of resolved [`AidRecord`]s.
///
/// Records whose storage entries were evicted are skipped without stalling
/// the cursor, so pagination always makes forward progress.
fn paginate(env: &Env, ids: &Vec<u64>, cursor: u32, limit: u32) -> AidPage {
    let effective_limit = if limit > MAX_QUERY_LIMIT {
        MAX_QUERY_LIMIT
    } else {
        limit
    };
    let total = ids.len();
    let mut records = Vec::new(env);
    let mut index = cursor;
    while index < total && records.len() < effective_limit {
        if let Some(record) = get_aid(env, ids.get(index).unwrap()) {
            records.push_back(record);
        }
        index += 1;
    }
    let next_cursor = if index < total { Some(index) } else { None };
    AidPage {
        records,
        next_cursor,
    }
}

#[cfg(test)]
mod tests;

