#![no_std]

use soroban_sdk::{contract, contractimpl, contracterror, token, Address, Env};

use shared::storage::{is_paused, set_paused as shared_set_paused};

pub mod storage;
pub mod types;

use storage::{get_aid, get_aid_counter, has_aid, set_aid, set_aid_counter};

pub use types::{AidRecord, AidStatus};

// ---------------------------------------------------------------------------
// Contract-specific error codes (range 100-199 per shared conventions)
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
    /// The expiry has not yet passed (refund attempted too early).
    NotExpiredYet = 105,
    /// The aid has already been refunded.
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

    /// Initialise the contract, storing the admin address and token.
    ///
    /// Must be called exactly once immediately after deployment.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        shared::auth::set_admin(&env, &admin);
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
    /// Returns the newly allocated `aid_id`.
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
            env.panic_with_error(shared::Error::InvalidAmount);
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

        // Checks-effects-interactions: write status first, then transfer.
        record.status = AidStatus::Settled;
        set_aid(&env, aid_id, &record);

        token::Client::new(&env, &record.token).transfer(
            &env.current_contract_address(),
            &record.recipient,
            &record.amount,
        );

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

        // Checks-effects-interactions.
        record.status = AidStatus::Refunded;
        set_aid(&env, aid_id, &record);

        token::Client::new(&env, &record.token).transfer(
            &env.current_contract_address(),
            &record.donor,
            &record.amount,
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Return the aid record for `aid_id`, or `None` if it does not exist.
    pub fn get_aid(env: Env, aid_id: u64) -> Option<AidRecord> {
        storage::get_aid(&env, aid_id)
    }

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
        shared_set_paused(&env, paused);
    }
}

#[cfg(test)]
mod tests;

