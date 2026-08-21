#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, token, Address, Env, Vec};

use shared::{emit, AID_CLAIMED, AID_CREATED, AID_REFUNDED, AID_SETTLED};
use shared::storage::{is_paused, set_paused as shared_set_paused};

pub mod storage;
pub mod types;

use storage::{
    append_donor_index, append_recipient_index, get_aid, get_aid_counter, get_donor_index,
    get_recipient_index, get_token, set_aid, set_aid_counter, set_token,
};

// Re-export so test modules (and `use super::*`) have access.
pub use types::{AidPage, AidRecord, AidStatus};

/// Upper bound for the `limit` argument of paginated queries.
///
/// Requests above this value are silently clamped so a single call can never
/// read an unbounded number of records; this bounds the CPU/memory footprint
/// (and therefore gas) of every page.  `limit` must be `> 0`.
pub const MAX_QUERY_LIMIT: u32 = 50;

// ---------------------------------------------------------------------------
// Contract-specific error codes  (range 100-199 per shared/README.md)
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AidError {
    /// Caller is not the authorised recipient / donor / admin.
    Unauthorized = 100,
    /// The requested aid record was not found.
    NotFound = 101,
    /// The aid has already been settled.
    AlreadyClaimed = 102,
    /// The claim window has expired (past `expiry_ledger`).
    Expired = 103,
    /// The contract is paused; no state-changing operations are allowed.
    Paused = 104,
    /// The aid has not expired yet and cannot be refunded.
    NotExpiredYet = 105,
    /// The aid has already been refunded to the donor.
    AlreadyRefunded = 106,
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

    /// Initialise the contract with an admin and the escrow token.
    ///
    /// Must be called exactly once immediately after deployment.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        shared::auth::set_admin(&env, &admin);
        set_token(&env, &token);
    }

    // -----------------------------------------------------------------------
    // Aid creation
    // -----------------------------------------------------------------------

    /// Create a new aid disbursement and escrow funds from the donor.
    ///
    /// Transfers `amount` of the configured token from `donor` into this
    /// contract for safekeeping until the recipient claims or the aid
    /// expires.  The `aid_id` is allocated from a monotonically-increasing
    /// counter and returned.
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

        if amount <= 0 {
            env.panic_with_error(shared::Error::InvalidAmount);
        }
        if expiry_ledger <= env.ledger().sequence() {
            env.panic_with_error(shared::Error::InvalidArgument);
        }
        let token = get_token(&env).expect("token not configured");

        // Escrow funds from donor into contract.
        token::Client::new(&env, &token).transfer(
            &donor,
            &env.current_contract_address(),
            &amount,
        );

        let aid_id = get_aid_counter(&env) + 1;
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

        append_donor_index(&env, &donor, aid_id);
        append_recipient_index(&env, &recipient, aid_id);

        emit(
            &env,
            AID_CREATED,
            (aid_id, donor, recipient, amount, expiry_ledger),
        );
        aid_id
    }

    // -----------------------------------------------------------------------
    // Aid claiming
    // -----------------------------------------------------------------------

    /// Claim a pending aid disbursement and transfer funds to the recipient.
    ///
    /// # Errors
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
    /// - [`AidError::Unauthorized`]   — `caller` is neither donor nor admin.
    /// - [`AidError::AlreadyClaimed`] — already settled.
    /// - [`AidError::AlreadyRefunded`]— already refunded.
    /// - [`AidError::NotExpiredYet`]  — expiry ledger has not passed.
    pub fn refund_aid(env: Env, caller: Address, aid_id: u64) -> Result<(), AidError> {
        let mut record = get_aid(&env, aid_id).ok_or(AidError::NotFound)?;

        let admin = shared::auth::get_admin(&env);
        if caller != record.donor && caller != admin {
            return Err(AidError::Unauthorized);
        }
        caller.require_auth();

        if record.status == AidStatus::Settled {
            return Err(AidError::AlreadyClaimed);
        }
        if record.status == AidStatus::Refunded {
            return Err(AidError::AlreadyRefunded);
        }
        if env.ledger().sequence() <= record.expiry_ledger {
            return Err(AidError::NotExpiredYet);
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
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Return the aid record for `aid_id`, or `None` if it does not exist.
    pub fn get_aid(env: Env, aid_id: u64) -> Option<AidRecord> {
        storage::get_aid(&env, aid_id)
    }

    /// Page through all aids created by `donor`, oldest first.
    ///
    /// * `cursor` — offset into the donor's index; start at `0`.
    /// * `limit`  — page size; must be `> 0` and is clamped to
    ///   [`MAX_QUERY_LIMIT`].
    ///
    /// Returns up to `limit` records plus `next_cursor` (`None` when no
    /// further pages exist).
    ///
    /// # Panics
    /// Panics with [`shared::Error::InvalidArgument`] when `limit == 0`.
    pub fn get_aids_by_donor(env: Env, donor: Address, cursor: u32, limit: u32) -> AidPage {
        if limit == 0 {
            env.panic_with_error(shared::Error::InvalidArgument);
        }
        let ids = get_donor_index(&env, &donor);
        paginate(&env, &ids, cursor, limit)
    }

    /// Page through all aids assigned to `recipient`, oldest first.
    ///
    /// Same cursor/limit semantics as [`Self::get_aids_by_donor`].
    ///
    /// # Panics
    /// Panics with [`shared::Error::InvalidArgument`] when `limit == 0`.
    pub fn get_aids_by_recipient(env: Env, recipient: Address, cursor: u32, limit: u32) -> AidPage {
        if limit == 0 {
            env.panic_with_error(shared::Error::InvalidArgument);
        }
        let ids = get_recipient_index(&env, &recipient);
        paginate(&env, &ids, cursor, limit)
    }

    // -----------------------------------------------------------------------
    // Admin controls
    // -----------------------------------------------------------------------

    /// Pause or resume the contract.  Admin only.
    pub fn set_paused(env: Env, caller: Address, paused: bool) {
        shared::auth::require_admin(&env, &caller).expect("unauthorized");
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
