#![no_std]

mod errors;
mod events;
mod storage;
mod types;

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

use shared::events::emit_module_initialized;
use errors::OracleError;
use types::{FeedLatest, PriceSubmission};

#[cfg(test)]
mod tests;

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    /// Initialize the oracle contract with an admin.
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

    /// Register a new authorized submitter (admin only).
    pub fn register_submitter(
        env: Env,
        caller: Address,
        submitter: Address,
    ) -> Result<(), OracleError> {
        shared::auth::require_admin(&env, &caller)
            .map_err(|_| OracleError::Unauthorized)?;
        caller.require_auth();

        storage::register_submitter(&env, submitter.clone(), env.ledger().timestamp())?;
        events::emit_submitter_registered(&env, &submitter, env.ledger().timestamp());
        Ok(())
    }

    /// Deactivate an authorized submitter (admin only).
    pub fn deactivate_submitter(
        env: Env,
        caller: Address,
        submitter: Address,
    ) -> Result<(), OracleError> {
        shared::auth::require_admin(&env, &caller)
            .map_err(|_| OracleError::Unauthorized)?;
        caller.require_auth();

        storage::deactivate_submitter(&env, &submitter);
        events::emit_submitter_deactivated(&env, &submitter, env.ledger().timestamp());
        Ok(())
    }

    /// Submit a price for a feed (authorized submitters only).
    ///
    /// Includes replay protection (nonce-based) and staleness validation.
    pub fn submit_price(
        env: Env,
        submitter: Address,
        feed_id: Symbol,
        price: i128,
        decimals: u32,
        timestamp: u64,
        nonce: u64,
    ) -> Result<u64, OracleError> {
        submitter.require_auth();

        // Validate price and decimals
        if price < 0 {
            return Err(OracleError::InvalidPrice);
        }
        if decimals > 18 {
            return Err(OracleError::InvalidDecimals);
        }

        // Check submitter is authorized
        if !storage::is_submitter_active(&env, &submitter) {
            return Err(OracleError::SubmitterNotAuthorized);
        }

        // Replay protection: check nonce
        let expected_nonce = storage::get_nonce(&env, &submitter, &feed_id);
        if nonce != expected_nonce + 1 {
            return Err(OracleError::DuplicateSubmission);
        }

        // Staleness check: ensure timestamp is recent
        let current_time = env.ledger().timestamp();
        let staleness_window = storage::get_staleness_window(&env);
        if current_time > timestamp + staleness_window {
            return Err(OracleError::SubmissionStale);
        }

        // Generate submission
        let submission_id = storage::next_submission_id(&env)?;
        let submission = PriceSubmission {
            id: submission_id,
            submitter: submitter.clone(),
            feed_id: feed_id.clone(),
            price,
            decimals,
            timestamp,
            nonce,
        };

        // Store submission
        storage::append_submission(&env, &feed_id, &submission);

        // Update latest value
        let latest = FeedLatest {
            feed_id: feed_id.clone(),
            price,
            decimals,
            timestamp,
            submission_count: storage::get_feed_latest(&env, &feed_id)
                .map(|f| f.submission_count + 1)
                .unwrap_or(1),
        };
        storage::set_feed_latest(&env, &feed_id, &latest);

        // Update nonce
        storage::set_nonce(&env, &submitter, &feed_id, nonce);

        // Emit event
        events::emit_price_submitted(&env, &feed_id, price, &submitter, current_time);

        Ok(submission_id)
    }

    /// Get the latest price for a feed.
    pub fn get_latest_price(env: Env, feed_id: Symbol) -> Result<FeedLatest, OracleError> {
        storage::get_feed_latest(&env, &feed_id).ok_or(OracleError::FeedNotFound)
    }

    /// Get submission history for a feed (latest N submissions).
    pub fn get_price_history(
        env: Env,
        feed_id: Symbol,
        limit: u32,
    ) -> Result<soroban_sdk::Vec<PriceSubmission>, OracleError> {
        if storage::get_feed_latest(&env, &feed_id).is_none() {
            return Err(OracleError::FeedNotFound);
        }
        Ok(storage::get_feed_history(&env, &feed_id, limit))
    }

    /// Set the staleness window (admin only).
    pub fn set_staleness_window(
        env: Env,
        caller: Address,
        seconds: u64,
    ) -> Result<(), OracleError> {
        shared::auth::require_admin(&env, &caller)
            .map_err(|_| OracleError::Unauthorized)?;
        caller.require_auth();

        storage::set_staleness_window(&env, seconds);
        events::emit_staleness_window_set(&env, seconds, env.ledger().timestamp());
        Ok(())
    }

    /// Get the current staleness window.
    pub fn get_staleness_window(env: Env) -> u64 {
        storage::get_staleness_window(&env)
    }

    /// Check if a submitter is active.
    pub fn is_submitter_active(env: Env, submitter: Address) -> bool {
        storage::is_submitter_active(&env, &submitter)
    }

    /// Get the admin address.
    pub fn get_admin(env: Env) -> Address {
        shared::auth::get_admin(&env)
    }
}
