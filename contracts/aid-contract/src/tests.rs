#![cfg(test)]

extern crate std;

use super::*;
use shared::Error as SharedError;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Env,
};
use std::vec;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const MINT_AMOUNT: i128 = 1_000_000;

struct Fixture {
    env: Env,
    admin: Address,
    donor: Address,
    recipient: Address,
    token_addr: Address,
    contract_id: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let donor = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract(admin.clone());
    let asset_client = token::StellarAssetClient::new(&env, &token_addr);
    asset_client.mint(&donor, &MINT_AMOUNT);

    let contract_id = env.register_contract(None, AidContract);
    let client = AidContractClient::new(&env, &contract_id);
    client.initialize(&admin, &token_addr);

    Fixture {
        env,
        admin,
        donor,
        recipient,
        token_addr,
        contract_id,
    }
}

/// Create `count` aids of 100 units each from `donor` to `recipient`,
/// returning the allocated IDs in creation order.
fn create_aids(
    env: &Env,
    client: &AidContractClient,
    donor: &Address,
    recipient: &Address,
    count: u32,
) -> std::vec::Vec<u64> {
    let expiry = env.ledger().sequence() + 10_000;
    let mut ids = std::vec::Vec::with_capacity(count as usize);
    for _ in 0..count {
        ids.push(client.create_aid(donor, recipient, &100, &expiry));
    }
    ids
}

fn page_ids(page: &AidPage) -> std::vec::Vec<u64> {
    page.records.iter().map(|r| r.id).collect()
}

fn advance_ledger(env: &Env, delta: u32) {
    env.ledger().with_mut(|l| {
        l.sequence_number += delta;
    });
}

// ---------------------------------------------------------------------------
// Claim lifecycle
// ---------------------------------------------------------------------------

#[test]
fn claim_transfers_escrow_and_settles() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let token_client = token::Client::new(&fx.env, &fx.token_addr);

    let expiry = fx.env.ledger().sequence() + 100;
    let aid_id = client.create_aid(&fx.donor, &fx.recipient, &500, &expiry);
    assert_eq!(token_client.balance(&fx.contract_id), 500);

    client.claim_aid(&aid_id, &fx.recipient);

    assert_eq!(token_client.balance(&fx.recipient), 500);
    assert_eq!(token_client.balance(&fx.contract_id), 0);

    let record = client.get_aid(&aid_id).unwrap();
    assert_eq!(record.status, AidStatus::Settled);
}

#[test]
fn second_claim_returns_already_claimed() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);

    let expiry = fx.env.ledger().sequence() + 100;
    let aid_id = client.create_aid(&fx.donor, &fx.recipient, &500, &expiry);
    client.claim_aid(&aid_id, &fx.recipient);

    let result = client.try_claim_aid(&aid_id, &fx.recipient);
    assert_eq!(result, Err(Ok(AidError::AlreadyClaimed)));
}

#[test]
fn claim_after_expiry_is_rejected() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);

    let expiry = fx.env.ledger().sequence() + 100;
    let aid_id = client.create_aid(&fx.donor, &fx.recipient, &500, &expiry);

    advance_ledger(&fx.env, 101);

    let result = client.try_claim_aid(&aid_id, &fx.recipient);
    assert_eq!(result, Err(Ok(AidError::Expired)));
}

#[test]
fn claim_by_wrong_address_is_unauthorized() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let stranger = Address::generate(&fx.env);

    let expiry = fx.env.ledger().sequence() + 100;
    let aid_id = client.create_aid(&fx.donor, &fx.recipient, &500, &expiry);

    let result = client.try_claim_aid(&aid_id, &stranger);
    assert_eq!(result, Err(Ok(AidError::Unauthorized)));
}

#[test]
fn claim_while_paused_is_rejected() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);

    let expiry = fx.env.ledger().sequence() + 100;
    let aid_id = client.create_aid(&fx.donor, &fx.recipient, &500, &expiry);
    client.set_paused(&fx.admin, &true);

    let result = client.try_claim_aid(&aid_id, &fx.recipient);
    assert_eq!(result, Err(Ok(AidError::Paused)));
}

#[test]
fn create_aid_rejects_non_positive_amount_and_past_expiry() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);

    let expiry = fx.env.ledger().sequence() + 100;
    assert_eq!(
        client.try_create_aid(&fx.donor, &fx.recipient, &0, &expiry),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            SharedError::InvalidAmount as u32
        )))
    );

    let past = fx.env.ledger().sequence();
    assert_eq!(
        client.try_create_aid(&fx.donor, &fx.recipient, &100, &past),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            SharedError::InvalidArgument as u32
        )))
    );
}

// ---------------------------------------------------------------------------
// Refunds
// ---------------------------------------------------------------------------

#[test]
fn refund_aid_after_expiry_returns_funds_to_donor() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let token_client = token::Client::new(&fx.env, &fx.token_addr);

    let expiry = fx.env.ledger().sequence() + 100;
    let aid_id = client.create_aid(&fx.donor, &fx.recipient, &500, &expiry);
    advance_ledger(&fx.env, 101);

    client.refund_aid(&fx.donor, &aid_id);

    assert_eq!(token_client.balance(&fx.donor), MINT_AMOUNT);
    assert_eq!(token_client.balance(&fx.contract_id), 0);
    let record = client.get_aid(&aid_id).unwrap();
    assert_eq!(record.status, AidStatus::Refunded);
}

#[test]
fn refund_aid_before_expiry_is_rejected() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);

    let expiry = fx.env.ledger().sequence() + 100;
    let aid_id = client.create_aid(&fx.donor, &fx.recipient, &500, &expiry);

    let result = client.try_refund_aid(&fx.donor, &aid_id);
    assert_eq!(result, Err(Ok(AidError::NotExpiredYet)));
}

#[test]
fn refund_claimed_aid_is_rejected() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);

    let expiry = fx.env.ledger().sequence() + 100;
    let aid_id = client.create_aid(&fx.donor, &fx.recipient, &500, &expiry);
    client.claim_aid(&aid_id, &fx.recipient);
    advance_ledger(&fx.env, 101);

    let result = client.try_refund_aid(&fx.donor, &aid_id);
    assert_eq!(result, Err(Ok(AidError::AlreadyClaimed)));
}

#[test]
fn refund_refunded_aid_is_rejected() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);

    let expiry = fx.env.ledger().sequence() + 100;
    let aid_id = client.create_aid(&fx.donor, &fx.recipient, &500, &expiry);
    advance_ledger(&fx.env, 101);
    client.refund_aid(&fx.donor, &aid_id);

    let result = client.try_refund_aid(&fx.admin, &aid_id);
    assert_eq!(result, Err(Ok(AidError::AlreadyRefunded)));
}

#[test]
fn refund_by_stranger_is_unauthorized() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let stranger = Address::generate(&fx.env);

    let expiry = fx.env.ledger().sequence() + 100;
    let aid_id = client.create_aid(&fx.donor, &fx.recipient, &500, &expiry);
    advance_ledger(&fx.env, 101);

    let result = client.try_refund_aid(&stranger, &aid_id);
    assert_eq!(result, Err(Ok(AidError::Unauthorized)));
}

#[test]
fn refund_by_admin_is_successful() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let token_client = token::Client::new(&fx.env, &fx.token_addr);

    let expiry = fx.env.ledger().sequence() + 100;
    let aid_id = client.create_aid(&fx.donor, &fx.recipient, &500, &expiry);
    advance_ledger(&fx.env, 101);

    client.refund_aid(&fx.admin, &aid_id);

    assert_eq!(token_client.balance(&fx.donor), MINT_AMOUNT);
    let record = client.get_aid(&aid_id).unwrap();
    assert_eq!(record.status, AidStatus::Refunded);
}

// ---------------------------------------------------------------------------
// Single-record queries
// ---------------------------------------------------------------------------

#[test]
fn get_aid_unknown_id_returns_none() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);

    assert_eq!(client.get_aid(&9_999), None);
}

#[test]
fn get_aid_returns_full_record() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);

    let expiry = fx.env.ledger().sequence() + 100;
    let aid_id = client.create_aid(&fx.donor, &fx.recipient, &250, &expiry);

    let record = client.get_aid(&aid_id).expect("record should exist");
    assert_eq!(record.id, aid_id);
    assert_eq!(record.donor, fx.donor);
    assert_eq!(record.recipient, fx.recipient);
    assert_eq!(record.token, fx.token_addr);
    assert_eq!(record.amount, 250);
    assert_eq!(record.expiry_ledger, expiry);
    assert_eq!(record.status, AidStatus::Pending);
}

#[test]
fn aid_ids_are_unique_and_monotonic() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);

    let ids = create_aids(&fx.env, &client, &fx.donor, &fx.recipient, 5);
    let sorted = ids.clone();
    assert_eq!(ids, sorted);
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(*id, (i as u64) + 1);
    }
}

// ---------------------------------------------------------------------------
// Pagination — donor
// ---------------------------------------------------------------------------

#[test]
fn donor_pagination_single_page_when_under_limit() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let ids = create_aids(&fx.env, &client, &fx.donor, &fx.recipient, 3);

    let page = client.get_aids_by_donor(&fx.donor, &0, &10);
    assert_eq!(page_ids(&page), ids);
    assert_eq!(page.next_cursor, None);
}

#[test]
fn donor_pagination_walks_all_pages_in_order() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let ids = create_aids(&fx.env, &client, &fx.donor, &fx.recipient, 7);

    let page1 = client.get_aids_by_donor(&fx.donor, &0, &3);
    assert_eq!(page_ids(&page1), ids[0..3]);
    assert_eq!(page1.next_cursor, Some(3));

    let cursor = page1.next_cursor.unwrap();
    let page2 = client.get_aids_by_donor(&fx.donor, &cursor, &3);
    assert_eq!(page_ids(&page2), ids[3..6]);
    assert_eq!(page2.next_cursor, Some(6));

    let cursor = page2.next_cursor.unwrap();
    let page3 = client.get_aids_by_donor(&fx.donor, &cursor, &3);
    assert_eq!(page_ids(&page3), ids[6..7]);
    assert_eq!(page3.next_cursor, None);
}

#[test]
fn donor_pagination_exact_multiple_boundary_ends_with_empty_page() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let ids = create_aids(&fx.env, &client, &fx.donor, &fx.recipient, 4);

    let page1 = client.get_aids_by_donor(&fx.donor, &0, &2);
    assert_eq!(page_ids(&page1), ids[0..2]);
    assert_eq!(page1.next_cursor, Some(2));

    let page2 = client.get_aids_by_donor(&fx.donor, &2, &2);
    assert_eq!(page_ids(&page2), ids[2..4]);
    assert_eq!(page2.next_cursor, None);

    // A further request past the end returns an empty page.
    let page3 = client.get_aids_by_donor(&fx.donor, &4, &2);
    assert_eq!(page3.records.len(), 0);
    assert_eq!(page3.next_cursor, None);
}

#[test]
fn donor_pagination_caps_limit_at_max() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let total = MAX_QUERY_LIMIT + 3;
    let ids = create_aids(&fx.env, &client, &fx.donor, &fx.recipient, total);

    let page = client.get_aids_by_donor(&fx.donor, &0, &(MAX_QUERY_LIMIT * 4));
    assert_eq!(page.records.len(), MAX_QUERY_LIMIT);
    assert_eq!(page_ids(&page), ids[0..MAX_QUERY_LIMIT as usize]);
    assert_eq!(page.next_cursor, Some(MAX_QUERY_LIMIT));

    let rest = client.get_aids_by_donor(&fx.donor, &MAX_QUERY_LIMIT, &MAX_QUERY_LIMIT);
    assert_eq!(rest.records.len(), 3);
    assert_eq!(rest.next_cursor, None);
}

#[test]
fn pagination_rejects_zero_limit() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);

    assert_eq!(
        client.try_get_aids_by_donor(&fx.donor, &0, &0),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            SharedError::InvalidArgument as u32
        )))
    );
    assert_eq!(
        client.try_get_aids_by_recipient(&fx.recipient, &0, &0),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            SharedError::InvalidArgument as u32
        )))
    );
}

#[test]
fn donor_pagination_empty_for_unknown_donor() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let unknown = Address::generate(&fx.env);

    create_aids(&fx.env, &client, &fx.donor, &fx.recipient, 2);

    let page = client.get_aids_by_donor(&unknown, &0, &10);
    assert_eq!(page.records.len(), 0);
    assert_eq!(page.next_cursor, None);
}

#[test]
fn pagination_cursor_beyond_end_returns_empty_page() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    create_aids(&fx.env, &client, &fx.donor, &fx.recipient, 2);

    let page = client.get_aids_by_donor(&fx.donor, &1_000, &10);
    assert_eq!(page.records.len(), 0);
    assert_eq!(page.next_cursor, None);
}

// ---------------------------------------------------------------------------
// Pagination — recipient and cross-index isolation
// ---------------------------------------------------------------------------

#[test]
fn recipient_pagination_returns_records_in_order() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let ids = create_aids(&fx.env, &client, &fx.donor, &fx.recipient, 5);

    let page1 = client.get_aids_by_recipient(&fx.recipient, &0, &2);
    assert_eq!(page_ids(&page1), ids[0..2]);
    assert_eq!(page1.next_cursor, Some(2));

    let page2 = client.get_aids_by_recipient(&fx.recipient, &2, &10);
    assert_eq!(page_ids(&page2), ids[2..5]);
    assert_eq!(page2.next_cursor, None);
}

#[test]
fn recipient_pagination_empty_for_unknown_recipient() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let unknown = Address::generate(&fx.env);

    create_aids(&fx.env, &client, &fx.donor, &fx.recipient, 2);

    let page = client.get_aids_by_recipient(&unknown, &0, &10);
    assert_eq!(page.records.len(), 0);
    assert_eq!(page.next_cursor, None);
}

#[test]
fn donor_and_recipient_indexes_are_isolated() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let donor_two = Address::generate(&fx.env);
    let recipient_two = Address::generate(&fx.env);

    let asset_client = token::StellarAssetClient::new(&fx.env, &fx.token_addr);
    asset_client.mint(&donor_two, &MINT_AMOUNT);

    let d1_r1 = create_aids(&fx.env, &client, &fx.donor, &fx.recipient, 2);
    let d1_r2 = create_aids(&fx.env, &client, &fx.donor, &recipient_two, 1);
    let d2_r1 = create_aids(&fx.env, &client, &donor_two, &fx.recipient, 3);

    // Donor pages contain only their own aids.
    assert_eq!(page_ids(&client.get_aids_by_donor(&fx.donor, &0, &10)), [d1_r1.clone(), d1_r2.clone()].concat());
    assert_eq!(page_ids(&client.get_aids_by_donor(&donor_two, &0, &10)), d2_r1);

    // Recipient pages merge aids across donors.
    let r1_expected = [d1_r1, d2_r1.clone()].concat();
    assert_eq!(
        page_ids(&client.get_aids_by_recipient(&fx.recipient, &0, &10)),
        r1_expected
    );
    assert_eq!(
        page_ids(&client.get_aids_by_recipient(&recipient_two, &0, &10)),
        d1_r2
    );
}

#[test]
fn indexes_stay_consistent_across_claim_and_refund_transitions() {
    let fx = setup();
    let client = AidContractClient::new(&fx.env, &fx.contract_id);
    let donor_two = Address::generate(&fx.env);

    let asset_client = token::StellarAssetClient::new(&fx.env, &fx.token_addr);
    asset_client.mint(&donor_two, &MINT_AMOUNT);

    let claimed_id = client.create_aid(&fx.donor, &fx.recipient, &100, &(fx.env.ledger().sequence() + 100));
    let refunded_id = client.create_aid(&donor_two, &fx.recipient, &100, &(fx.env.ledger().sequence() + 100));
    let pending_id = client.create_aid(&fx.donor, &fx.recipient, &100, &(fx.env.ledger().sequence() + 100));

    // Transition one aid to Settled and another to Refunded.
    client.claim_aid(&claimed_id, &fx.recipient);
    advance_ledger(&fx.env, 101);
    client.refund_aid(&donor_two, &refunded_id);

    // Recipient index still lists all three IDs exactly once.
    let page = client.get_aids_by_recipient(&fx.recipient, &0, &10);
    assert_eq!(page_ids(&page), vec![claimed_id, refunded_id, pending_id]);

    // Statuses observed through the index reflect the transitions.
    assert_eq!(page.records.get(0).unwrap().status, AidStatus::Settled);
    assert_eq!(page.records.get(1).unwrap().status, AidStatus::Refunded);
    assert_eq!(page.records.get(2).unwrap().status, AidStatus::Pending);

    // Donor indexes unchanged by the transitions.
    assert_eq!(
        page_ids(&client.get_aids_by_donor(&fx.donor, &0, &10)),
        vec![claimed_id, pending_id]
    );
    assert_eq!(
        page_ids(&client.get_aids_by_donor(&donor_two, &0, &10)),
        vec![refunded_id]
    );
}
