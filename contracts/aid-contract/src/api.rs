
use soroban_sdk::{contracttype, contract, contractimpl, Env, Vec, Address};
use crate::types::AidRecord;
use crate::storage::{get_aid, get_aid_counter};

#[contracttype]
#[derive(Clone, Debug)]
pub struct PaginatedAidsResponse {
    pub aids: Vec<AidRecord>,
    pub next_cursor: Option<u64>,
}

#[contract]
pub struct ExternalApi;

#[contractimpl]
impl ExternalApi {
    pub fn list_aids(env: Env, limit: u32, cursor: Option<u64>) -> PaginatedAidsResponse {
        if limit == 0 {
            // Or return an error, depending on desired behavior for limit=0
            return PaginatedAidsResponse {
                aids: Vec::new(&env),
                next_cursor: cursor,
            };
        }

        let mut aids = Vec::new(&env);
        let total_aids = get_aid_counter(&env);
        let start = cursor.unwrap_or(0);

        let mut current_id = start;
        while aids.len() < limit && current_id < total_aids {
            if let Some(aid) = get_aid(&env, current_id) {
                aids.push_back(aid);
            }
            current_id += 1;
        }

        let next_cursor = if current_id < total_aids {
            Some(current_id)
        } else {
            None
        };

        PaginatedAidsResponse {
            aids,
            next_cursor,
        }
    }

    pub fn list_aids_by_donor(env: Env, donor: Address, limit: u32, cursor: Option<u64>) -> PaginatedAidsResponse {
        if limit == 0 {
            return PaginatedAidsResponse {
                aids: Vec::new(&env),
                next_cursor: cursor,
            };
        }

        let mut aids = Vec::new(&env);
        let total_aids = get_aid_counter(&env);
        let start = cursor.unwrap_or(0);

        let mut current_id = start;
        while aids.len() < limit && current_id < total_aids {
            if let Some(aid) = get_aid(&env, current_id) {
                if aid.donor == donor {
                    aids.push_back(aid);
                }
            }
            current_id += 1;
        }

        let next_cursor = if current_id < total_aids {
            Some(current_id)
        } else {
            None
        };

        PaginatedAidsResponse {
            aids,
            next_cursor,
        }
    }
}