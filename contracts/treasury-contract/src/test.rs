use crate::{TreasuryContract, TreasuryContractClient};
use shared::errors::Error;
use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Events},
    Address, Env,
};

fn setup(env: &Env) -> (TreasuryContractClient<'static>, Address, i128) {
    let contract_id = env.register_contract(None, TreasuryContract);
    let client = TreasuryContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let limit: i128 = 1_000;
    client.initialize(&admin, &limit);
    (client, admin, limit)
}

#[test]
fn test_withdraw_success_decrements_balance_and_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let category = symbol_short!("reserve");
    let recipient = Address::generate(&env);

    client.deposit(&admin, &category, &500);

    client.withdraw(&admin, &recipient, &200, &category);

    assert!(
        !env.events().all().is_empty(),
        "expected TREASURY_WITHDRAW event to be emitted"
    );
    assert_eq!(client.category_balance(&category), 300);
}

#[test]
fn test_withdraw_rejects_non_manager() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let category = symbol_short!("reserve");
    let recipient = Address::generate(&env);
    let stranger = Address::generate(&env);

    client.deposit(&admin, &category, &500);

    let result = client.try_withdraw(&stranger, &recipient, &100, &category);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_withdraw_rejects_amount_above_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, limit) = setup(&env);
    let category = symbol_short!("reserve");
    let recipient = Address::generate(&env);

    client.deposit(&admin, &category, &(limit * 2));

    let over_limit = limit + 1;
    let result = client.try_withdraw(&admin, &recipient, &over_limit, &category);
    assert_eq!(result, Err(Ok(Error::WithdrawalLimitExceeded)));
}

#[test]
fn test_withdraw_rejects_insufficient_category_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let category = symbol_short!("rewards");
    let recipient = Address::generate(&env);

    client.deposit(&admin, &category, &50);

    let result = client.try_withdraw(&admin, &recipient, &100, &category);
    assert_eq!(result, Err(Ok(Error::InsufficientBalance)));
}

#[test]
fn test_withdraw_rejects_zero_or_negative_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let category = symbol_short!("reserve");
    let recipient = Address::generate(&env);

    client.deposit(&admin, &category, &500);

    let result = client.try_withdraw(&admin, &recipient, &0, &category);
    assert_eq!(result, Err(Ok(Error::InvalidArgument)));
}

#[test]
fn test_admin_can_add_and_remove_treasury_manager() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let category = symbol_short!("reserve");
    let recipient = Address::generate(&env);
    let manager = Address::generate(&env);

    client.deposit(&admin, &category, &500);

    // Not yet a manager -> rejected.
    let result = client.try_withdraw(&manager, &recipient, &100, &category);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));

    // Admin grants the role -> now allowed.
    client.add_treasury_manager(&admin, &manager);
    client.withdraw(&manager, &recipient, &100, &category);
    assert_eq!(client.category_balance(&category), 400);

    // Admin revokes the role -> rejected again.
    client.remove_treasury_manager(&admin, &manager);
    let result = client.try_withdraw(&manager, &recipient, &50, &category);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

/// Stands in for the referral contract: it forwards to the treasury's
/// `distribute_reward`, so from the treasury's perspective the direct caller
/// is this contract's own address (whichever instance is registered).
#[contract]
struct MockReferralCaller;

#[contractimpl]
impl MockReferralCaller {
    pub fn call_distribute(
        env: Env,
        treasury: Address,
        recipient: Address,
        amount: i128,
    ) -> Result<(), Error> {
        let client = TreasuryContractClient::new(&env, &treasury);
        match client.try_distribute_reward(&recipient, &amount) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(Error::InvalidArgument),
            Err(Ok(error)) => Err(error),
            Err(Err(_)) => Err(Error::InvalidArgument),
        }
    }
}

#[test]
fn test_distribute_reward_pays_recipient_and_decrements_rewards() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let rewards = symbol_short!("rewards");
    let recipient = Address::generate(&env);
    let referral_id = env.register_contract(None, MockReferralCaller);
    let referral = MockReferralCallerClient::new(&env, &referral_id);

    client.deposit(&admin, &rewards, &1_000);
    client.set_referral_contract(&admin, &referral_id);

    referral.call_distribute(&client.address, &recipient, &400);

    assert!(
        !env.events().all().is_empty(),
        "expected CommissionPaid event to be emitted"
    );
    assert_eq!(client.category_balance(&rewards), 600);
}

#[test]
fn test_distribute_reward_rejects_underfunded_rewards_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let rewards = symbol_short!("rewards");
    let recipient = Address::generate(&env);
    let referral_id = env.register_contract(None, MockReferralCaller);
    let referral = MockReferralCallerClient::new(&env, &referral_id);

    client.deposit(&admin, &rewards, &100);
    client.set_referral_contract(&admin, &referral_id);

    let result = referral.try_call_distribute(&client.address, &recipient, &200);
    assert_eq!(result, Err(Ok(Error::InsufficientBalance)));
    assert_eq!(client.category_balance(&rewards), 100);
}

#[test]
fn test_distribute_reward_rejects_caller_that_is_not_registered_referral_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let rewards = symbol_short!("rewards");
    let recipient = Address::generate(&env);
    let referral_id = env.register_contract(None, MockReferralCaller);
    let impostor_id = env.register_contract(None, MockReferralCaller);
    let impostor = MockReferralCallerClient::new(&env, &impostor_id);

    client.deposit(&admin, &rewards, &1_000);
    client.set_referral_contract(&admin, &referral_id);

    // Disable blanket auth mocking so the invoker check below is genuinely
    // enforced instead of auto-approved for every address.
    env.set_auths(&[]);

    let result = impostor.try_call_distribute(&client.address, &recipient, &400);
    assert!(result.is_err());
    assert_eq!(client.category_balance(&rewards), 1_000);
}

#[test]
fn test_distribute_reward_rejects_when_no_referral_contract_registered() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let rewards = symbol_short!("rewards");
    let recipient = Address::generate(&env);
    let referral_id = env.register_contract(None, MockReferralCaller);
    let referral = MockReferralCallerClient::new(&env, &referral_id);

    client.deposit(&admin, &rewards, &1_000);

    let result = referral.try_call_distribute(&client.address, &recipient, &400);
    assert!(result.is_err());
    assert_eq!(client.category_balance(&rewards), 1_000);
}

// ===========================================================================
// Gas benchmark tests
// ===========================================================================

/// Benchmark: withdraw rejects zero-amount BEFORE auth check.
///
/// Before optimization: `require_role` ran first (auth commit).
/// After: `amount <= 0` check runs first — saves the auth cost on trivial
/// rejects.
#[test]
fn gas_bench_withdraw_rejects_zero_before_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let category = symbol_short!("reserve");
    let recipient = Address::generate(&env);

    client.deposit(&admin, &category, &500);

    // Zero amount rejected before auth commit — cheaper error path
    let stranger = Address::generate(&env);
    let result = client.try_withdraw(&stranger, &recipient, &0, &category);
    assert_eq!(result, Err(Ok(Error::InvalidArgument)));
}

/// Benchmark: deposit rejects negative amount BEFORE auth check.
#[test]
fn gas_bench_deposit_rejects_zero_before_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let category = symbol_short!("reserve");
    let stranger = Address::generate(&env);

    let result = client.try_deposit(&stranger, &category, &0);
    assert_eq!(result, Err(Ok(Error::InvalidArgument)));
}

/// Benchmark: distribute_reward rejects zero BEFORE referral lookup + auth.
#[test]
fn gas_bench_distribute_reward_rejects_zero_before_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let rewards = symbol_short!("rewards");
    let recipient = Address::generate(&env);

    client.deposit(&admin, &rewards, &1_000);

    let result = client.try_distribute_reward(&recipient, &0);
    assert_eq!(result, Err(Ok(Error::InvalidArgument)));
}

/// Benchmark: emergency_withdraw rejects zero BEFORE pause check + auth.
#[test]
fn gas_bench_emergency_withdraw_rejects_zero_before_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _limit) = setup(&env);
    let recipient = Address::generate(&env);

    let result = client.try_emergency_withdraw(&admin, &recipient, &0);
    assert_eq!(result, Err(Ok(Error::InvalidArgument)));
}
