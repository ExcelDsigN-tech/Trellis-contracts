use soroban_sdk::{
    contract, contractimpl, panic_with_error, token, Address, Env,
};

use shared::{
    auth,
    storage::{is_paused, set_paused as shared_set_paused},
    Error as SharedError,
    event::{emit, AID_CLAIMED, AID_CREATED, AID_REFUNDED, AID_SETTLED},
};

pub mod storage;
pub mod types;
pub mod api;

use storage::{get_aid, has_aid, set_aid, get_aid_counter, set_aid_counter};
use types::{AidRecord, AidStatus};
pub use api::{ExternalApi, PaginatedAidsResponse};


#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AidError {
    Unauthorized = 100,
    NotFound = 101,
    AlreadyClaimed = 102,
    Expired = 103,
    Paused = 104,
}

#[contract]
pub struct AidContract;

#[contractimpl]
impl AidContract {
    pub fn initialize(env: Env, admin: Address) {
        auth::set_admin(&env, &admin);
    }

    pub fn create_aid(
        env: Env,
        donor: Address,
        recipient: Address,
        token: Address,
        amount: i128,
        expiry_ledger: u32,
    ) -> u64 {
        donor.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, SharedError::InvalidAmount);
        }
        if expiry_ledger <= env.ledger().sequence() {
            panic_with_error!(&env, SharedError::InvalidArgument);
        }

        let aid_id = get_aid_counter(&env);
        set_aid_counter(&env, aid_id + 1);

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

        emit(&env, AID_CREATED, (aid_id, &donor, &recipient, amount, expiry_ledger));
        aid_id
    }

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

        record.status = AidStatus::Settled;
        set_aid(&env, aid_id, &record);

        token::Client::new(&env, &record.token).transfer(
            &env.current_contract_address(),
            &record.recipient,
            &record.amount,
        );

        emit(&env, AID_CLAIMED, (aid_id,));
        emit(&env, AID_SETTLED, (aid_id,));
        Ok(())
    }

    pub fn refund_expired(env: Env, aid_id: u64) -> Result<(), AidError> {
        let mut record = get_aid(&env, aid_id).ok_or(AidError::NotFound)?;

        if record.status != AidStatus::Pending {
            return Err(AidError::AlreadyClaimed);
        }
        if env.ledger().sequence() <= record.expiry_ledger {
            panic_with_error!(&env, SharedError::InvalidArgument);
        }

        record.status = AidStatus::Refunded;
        set_aid(&env, aid_id, &record);

        token::Client::new(&env, &record.token).transfer(
            &env.current_contract_address(),
            &record.donor,
            &record.amount,
        );

        emit(&env, AID_REFUNDED, (aid_id,));
        Ok(())
    }

    pub fn get_aid(env: Env, aid_id: u64) -> Option<AidRecord> {
        storage::get_aid(&env, aid_id)
    }

    pub fn set_paused(env: Env, caller: Address, paused: bool) {
        auth::require_admin(&env, &caller).expect("unauthorized");
        shared_set_paused(&env, paused);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_api;