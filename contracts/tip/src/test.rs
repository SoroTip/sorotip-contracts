#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    token, Address, Env, IntoVal, String,
};

use crate::{types::SECONDS_PER_MONTH, TipContract, TipContractClient};

struct TestCtx<'a> {
    env: Env,
    contract: TipContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    admin: Address,
    treasury: Address,
}

fn setup(fee_basis_points: u32) -> TestCtx<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin_addr = Address::generate(&env);

    let token_contract_id = env.register_stellar_asset_contract_v2(token_admin_addr.clone());
    let token = token::Client::new(&env, &token_contract_id.address());
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id.address());

    let contract_id = env.register(TipContract, ());
    let contract = TipContractClient::new(&env, &contract_id);
    contract.initialize(&admin, &fee_basis_points, &treasury, &token.address);

    TestCtx {
        env,
        contract,
        token,
        token_admin,
        admin,
        treasury,
    }
}

fn name(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

#[test]
fn test_register_creator_success() {
    let ctx = setup(0);
    let creator = Address::generate(&ctx.env);

    let id = ctx.contract.register_creator(
        &creator,
        &name(&ctx.env, "Ada"),
        &name(&ctx.env, "Building on Stellar"),
        &name(&ctx.env, "ipfs://avatar"),
    );

    assert_eq!(id, 1);
    let profile = ctx.contract.get_profile(&creator);
    assert_eq!(profile.wallet, creator);
    assert_eq!(profile.name, name(&ctx.env, "Ada"));
    assert_eq!(profile.total_received, 0);
    assert_eq!(profile.tip_count, 0);
}

#[test]
fn test_update_profile() {
    let ctx = setup(0);
    let creator = Address::generate(&ctx.env);

    ctx.contract.register_creator(
        &creator,
        &name(&ctx.env, "Ada"),
        &name(&ctx.env, "Old bio"),
        &name(&ctx.env, "ipfs://old"),
    );
    ctx.contract.update_profile(
        &creator,
        &name(&ctx.env, "Ada Lovelace"),
        &name(&ctx.env, "New bio"),
        &name(&ctx.env, "ipfs://new"),
    );

    let profile = ctx.contract.get_profile(&creator);
    assert_eq!(profile.name, name(&ctx.env, "Ada Lovelace"));
    assert_eq!(profile.bio, name(&ctx.env, "New bio"));
    assert_eq!(profile.avatar_ipfs, name(&ctx.env, "ipfs://new"));
}

#[test]
fn test_tip_direct_transfer() {
    let ctx = setup(0);
    let supporter = Address::generate(&ctx.env);
    let creator = Address::generate(&ctx.env);

    ctx.token_admin.mint(&supporter, &10_000_000_000);
    ctx.contract.register_creator(
        &creator,
        &name(&ctx.env, "Bob"),
        &name(&ctx.env, "bio"),
        &name(&ctx.env, ""),
    );

    ctx.contract
        .tip(&supporter, &creator, &100_0000000, &name(&ctx.env, ""));

    assert_eq!(ctx.token.balance(&creator), 100_0000000);
    assert_eq!(ctx.token.balance(&supporter), 900_0000000);
}

#[test]
fn test_tip_with_fee() {
    let ctx = setup(250); // 2.5%
    let supporter = Address::generate(&ctx.env);
    let creator = Address::generate(&ctx.env);

    ctx.token_admin.mint(&supporter, &10_000_000_000);
    ctx.contract.register_creator(
        &creator,
        &name(&ctx.env, "Bob"),
        &name(&ctx.env, "bio"),
        &name(&ctx.env, ""),
    );

    ctx.contract
        .tip(&supporter, &creator, &100_0000000, &name(&ctx.env, ""));

    let expected_fee = 100_0000000i128 * 250 / 10_000;
    assert_eq!(ctx.token.balance(&ctx.treasury), expected_fee);
    assert_eq!(ctx.token.balance(&creator), 100_0000000 - expected_fee);
}

#[test]
fn test_tip_without_fee() {
    let ctx = setup(0);
    let supporter = Address::generate(&ctx.env);
    let creator = Address::generate(&ctx.env);

    ctx.token_admin.mint(&supporter, &500_0000000);
    ctx.contract.register_creator(
        &creator,
        &name(&ctx.env, "Bob"),
        &name(&ctx.env, "bio"),
        &name(&ctx.env, ""),
    );

    ctx.contract
        .tip(&supporter, &creator, &50_0000000, &name(&ctx.env, ""));

    assert_eq!(ctx.token.balance(&ctx.treasury), 0);
    assert_eq!(ctx.token.balance(&creator), 50_0000000);
}

#[test]
fn test_tip_message_stored() {
    let ctx = setup(0);
    let supporter = Address::generate(&ctx.env);
    let creator = Address::generate(&ctx.env);

    ctx.token_admin.mint(&supporter, &100_0000000);
    ctx.contract.register_creator(
        &creator,
        &name(&ctx.env, "Bob"),
        &name(&ctx.env, "bio"),
        &name(&ctx.env, ""),
    );
    ctx.contract.tip(
        &supporter,
        &creator,
        &10_0000000,
        &name(&ctx.env, "ipfs://message-hash"),
    );

    let history = ctx.contract.get_tip_history(&creator, &10);
    assert_eq!(history.len(), 1);
    assert_eq!(
        history.get(0).unwrap().message_ipfs,
        name(&ctx.env, "ipfs://message-hash")
    );
}

#[test]
fn test_subscribe_success() {
    let ctx = setup(0);
    let supporter = Address::generate(&ctx.env);
    let creator = Address::generate(&ctx.env);
    ctx.contract.register_creator(
        &creator,
        &name(&ctx.env, "Bob"),
        &name(&ctx.env, "bio"),
        &name(&ctx.env, ""),
    );

    let id = ctx.contract.subscribe(&supporter, &creator, &5_0000000);
    assert_eq!(id, 1);

    let subs = ctx.contract.get_subscriptions_by_supporter(&supporter);
    assert_eq!(subs.len(), 1);
    let sub = subs.get(0).unwrap();
    assert!(sub.active);
    assert_eq!(sub.amount_per_month, 5_0000000);

    let profile = ctx.contract.get_profile(&creator);
    assert_eq!(profile.subscriber_count, 1);
}

#[test]
fn test_subscription_charged_on_schedule() {
    let ctx = setup(0);
    let supporter = Address::generate(&ctx.env);
    let creator = Address::generate(&ctx.env);
    ctx.contract.register_creator(
        &creator,
        &name(&ctx.env, "Bob"),
        &name(&ctx.env, "bio"),
        &name(&ctx.env, ""),
    );

    ctx.token_admin.mint(&supporter, &10_000_000_000);
    ctx.token.approve(
        &supporter,
        &ctx.contract.address,
        &100_0000000,
        &(SECONDS_PER_MONTH as u32 + 1000),
    );

    ctx.contract.subscribe(&supporter, &creator, &10_0000000);

    // Nothing due yet.
    let charged = ctx.contract.process_due_subscriptions();
    assert_eq!(charged, 0);
    assert_eq!(ctx.token.balance(&creator), 0);

    ctx.env
        .ledger()
        .with_mut(|li| li.timestamp += SECONDS_PER_MONTH);

    let charged = ctx.contract.process_due_subscriptions();
    assert_eq!(charged, 1);
    assert_eq!(ctx.token.balance(&creator), 10_0000000);

    let subs = ctx.contract.get_subscriptions_by_creator(&creator);
    let sub = subs.get(0).unwrap();
    assert!(sub.next_charge_date > ctx.env.ledger().timestamp());
}

#[test]
fn test_cancel_subscription() {
    let ctx = setup(0);
    let supporter = Address::generate(&ctx.env);
    let creator = Address::generate(&ctx.env);
    ctx.contract.register_creator(
        &creator,
        &name(&ctx.env, "Bob"),
        &name(&ctx.env, "bio"),
        &name(&ctx.env, ""),
    );

    let id = ctx.contract.subscribe(&supporter, &creator, &5_0000000);
    ctx.contract.cancel_subscription(&supporter, &id);

    let subs = ctx.contract.get_subscriptions_by_supporter(&supporter);
    assert!(!subs.get(0).unwrap().active);

    let profile = ctx.contract.get_profile(&creator);
    assert_eq!(profile.subscriber_count, 0);

    let result = ctx.contract.try_cancel_subscription(&supporter, &id);
    assert!(result.is_err());
}

#[test]
fn test_process_due_subscriptions_charges_correct_wallets() {
    let ctx = setup(0);
    let creator_a = Address::generate(&ctx.env);
    let creator_b = Address::generate(&ctx.env);
    let supporter_a = Address::generate(&ctx.env);
    let supporter_b = Address::generate(&ctx.env);

    ctx.contract.register_creator(
        &creator_a,
        &name(&ctx.env, "A"),
        &name(&ctx.env, ""),
        &name(&ctx.env, ""),
    );
    ctx.contract.register_creator(
        &creator_b,
        &name(&ctx.env, "B"),
        &name(&ctx.env, ""),
        &name(&ctx.env, ""),
    );

    ctx.token_admin.mint(&supporter_a, &10_000_000_000);
    ctx.token_admin.mint(&supporter_b, &10_000_000_000);
    ctx.token.approve(
        &supporter_a,
        &ctx.contract.address,
        &10_000_000_000,
        &(SECONDS_PER_MONTH as u32 + 1000),
    );
    ctx.token.approve(
        &supporter_b,
        &ctx.contract.address,
        &10_000_000_000,
        &(SECONDS_PER_MONTH as u32 + 1000),
    );

    ctx.contract
        .subscribe(&supporter_a, &creator_a, &20_0000000);
    ctx.contract
        .subscribe(&supporter_b, &creator_b, &30_0000000);

    ctx.env
        .ledger()
        .with_mut(|li| li.timestamp += SECONDS_PER_MONTH);
    let charged = ctx.contract.process_due_subscriptions();

    assert_eq!(charged, 2);
    assert_eq!(ctx.token.balance(&creator_a), 20_0000000);
    assert_eq!(ctx.token.balance(&creator_b), 30_0000000);
    assert_eq!(ctx.token.balance(&supporter_a), 980_0000000);
    assert_eq!(ctx.token.balance(&supporter_b), 970_0000000);
}

#[test]
fn test_set_and_reach_tip_goal() {
    let ctx = setup(0);
    let creator = Address::generate(&ctx.env);
    let supporter = Address::generate(&ctx.env);

    ctx.contract.register_creator(
        &creator,
        &name(&ctx.env, "Bob"),
        &name(&ctx.env, "bio"),
        &name(&ctx.env, ""),
    );
    let goal_id =
        ctx.contract
            .set_tip_goal(&creator, &100_0000000, &name(&ctx.env, "New microphone"));

    let goal = ctx.contract.get_tip_goal(&creator).unwrap();
    assert_eq!(goal.id, goal_id);
    assert!(!goal.completed);

    ctx.token_admin.mint(&supporter, &200_0000000);
    ctx.contract
        .tip(&supporter, &creator, &60_0000000, &name(&ctx.env, ""));
    assert!(!ctx.contract.get_tip_goal(&creator).unwrap().completed);

    ctx.contract
        .tip(&supporter, &creator, &50_0000000, &name(&ctx.env, ""));
    let goal = ctx.contract.get_tip_goal(&creator).unwrap();
    assert!(goal.completed);
    assert!(goal.current_amount >= goal.goal_amount);
}

#[test]
fn test_get_top_creators_ordered_by_volume() {
    let ctx = setup(0);
    let creator_a = Address::generate(&ctx.env);
    let creator_b = Address::generate(&ctx.env);
    let creator_c = Address::generate(&ctx.env);
    let supporter = Address::generate(&ctx.env);

    ctx.contract.register_creator(
        &creator_a,
        &name(&ctx.env, "A"),
        &name(&ctx.env, ""),
        &name(&ctx.env, ""),
    );
    ctx.contract.register_creator(
        &creator_b,
        &name(&ctx.env, "B"),
        &name(&ctx.env, ""),
        &name(&ctx.env, ""),
    );
    ctx.contract.register_creator(
        &creator_c,
        &name(&ctx.env, "C"),
        &name(&ctx.env, ""),
        &name(&ctx.env, ""),
    );

    ctx.token_admin.mint(&supporter, &10_000_000_000);
    ctx.contract
        .tip(&supporter, &creator_a, &10_0000000, &name(&ctx.env, ""));
    ctx.contract
        .tip(&supporter, &creator_b, &50_0000000, &name(&ctx.env, ""));
    ctx.contract
        .tip(&supporter, &creator_c, &30_0000000, &name(&ctx.env, ""));

    let top = ctx.contract.get_top_creators(&2);
    assert_eq!(top.len(), 2);
    assert_eq!(top.get(0).unwrap().wallet, creator_b);
    assert_eq!(top.get(1).unwrap().wallet, creator_c);
}

#[test]
fn test_protocol_stats_accurate() {
    let ctx = setup(100); // 1%
    let creator = Address::generate(&ctx.env);
    let supporter = Address::generate(&ctx.env);

    ctx.contract.register_creator(
        &creator,
        &name(&ctx.env, "Bob"),
        &name(&ctx.env, "bio"),
        &name(&ctx.env, ""),
    );
    ctx.token_admin.mint(&supporter, &10_000_000_000);

    ctx.contract
        .tip(&supporter, &creator, &100_0000000, &name(&ctx.env, ""));
    ctx.contract
        .tip(&supporter, &creator, &50_0000000, &name(&ctx.env, ""));
    ctx.contract.subscribe(&supporter, &creator, &5_0000000);

    let stats = ctx.contract.get_protocol_stats();
    assert_eq!(stats.total_tips, 2);
    assert_eq!(stats.total_volume, 150_0000000);
    assert_eq!(stats.total_creators, 1);
    assert_eq!(stats.total_subscriptions, 1);
    assert_eq!(stats.fee_collected, 150_0000000i128 * 100 / 10_000);
}

#[test]
fn test_tip_and_subscription_charge_never_leave_funds_in_contract() {
    let ctx = setup(250); // 2.5% fee, so both a tip and a charge route a second transfer.
    let supporter = Address::generate(&ctx.env);
    let creator = Address::generate(&ctx.env);
    ctx.contract.register_creator(
        &creator,
        &name(&ctx.env, "Bob"),
        &name(&ctx.env, "bio"),
        &name(&ctx.env, ""),
    );

    ctx.token_admin.mint(&supporter, &10_000_000_000);
    ctx.contract
        .tip(&supporter, &creator, &100_0000000, &name(&ctx.env, ""));
    assert_eq!(ctx.token.balance(&ctx.contract.address), 0);

    ctx.token.approve(
        &supporter,
        &ctx.contract.address,
        &100_0000000,
        &(SECONDS_PER_MONTH as u32 + 1000),
    );
    ctx.contract.subscribe(&supporter, &creator, &10_0000000);
    ctx.env
        .ledger()
        .with_mut(|li| li.timestamp += SECONDS_PER_MONTH);
    ctx.contract.process_due_subscriptions();
    assert_eq!(ctx.token.balance(&ctx.contract.address), 0);
}

/// Proves process_due_subscriptions really implements the keeper pattern: the
/// supporter authorizes a token allowance once (via `approve`), and the keeper
/// call that later spends it needs no authorization of its own. This test uses
/// `mock_auths` to mock only the specific calls that require a signature —
/// never `mock_all_auths` — so the final `process_due_subscriptions` call runs
/// with zero mocked (or real) authorizations. If the contract design relied on
/// a plain `transfer` instead of an allowance + `transfer_from`, this test
/// would fail with a missing-authorization error instead of moving funds.
#[test]
fn test_keeper_can_charge_subscription_using_only_preapproved_allowance() {
    let env = Env::default();

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin_addr = Address::generate(&env);
    let supporter = Address::generate(&env);
    let creator = Address::generate(&env);

    let token_contract_id = env.register_stellar_asset_contract_v2(token_admin_addr.clone());
    let token = token::Client::new(&env, &token_contract_id.address());
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id.address());

    let contract_id = env.register(TipContract, ());
    let contract = TipContractClient::new(&env, &contract_id);

    contract
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (&admin, 0u32, &treasury, token.address.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(&admin, &0, &treasury, &token.address);

    contract
        .mock_auths(&[MockAuth {
            address: &creator,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "register_creator",
                args: (
                    &creator,
                    name(&env, "Bob"),
                    name(&env, "bio"),
                    name(&env, ""),
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .register_creator(
            &creator,
            &name(&env, "Bob"),
            &name(&env, "bio"),
            &name(&env, ""),
        );

    token_admin
        .mock_auths(&[MockAuth {
            address: &token_admin_addr,
            invoke: &MockAuthInvoke {
                contract: &token_contract_id.address(),
                fn_name: "mint",
                args: (&supporter, 10_000_000_000i128).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .mint(&supporter, &10_000_000_000);

    token
        .mock_auths(&[MockAuth {
            address: &supporter,
            invoke: &MockAuthInvoke {
                contract: &token_contract_id.address(),
                fn_name: "approve",
                args: (
                    &supporter,
                    &contract_id,
                    100_0000000i128,
                    SECONDS_PER_MONTH as u32 + 1000,
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .approve(
            &supporter,
            &contract_id,
            &100_0000000,
            &(SECONDS_PER_MONTH as u32 + 1000),
        );

    contract
        .mock_auths(&[MockAuth {
            address: &supporter,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "subscribe",
                args: (&supporter, &creator, 10_0000000i128).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .subscribe(&supporter, &creator, &10_0000000);

    env.ledger()
        .with_mut(|li| li.timestamp += SECONDS_PER_MONTH);

    // No mock_auths and no mock_all_auths for this call: the keeper identity
    // calling process_due_subscriptions is unauthenticated, exactly as it
    // would be on a public network where anyone can invoke it.
    let charged = contract.process_due_subscriptions();

    assert_eq!(charged, 1);
    assert_eq!(token.balance(&creator), 10_0000000);
    assert_eq!(token.balance(&supporter), 10_000_000_000 - 10_0000000);
    assert_eq!(token.balance(&contract_id), 0);
}

#[test]
fn test_process_due_subscriptions_skips_not_yet_due() {
    let ctx = setup(0);
    let creator_due = Address::generate(&ctx.env);
    let creator_not_due = Address::generate(&ctx.env);
    let supporter_due = Address::generate(&ctx.env);
    let supporter_not_due = Address::generate(&ctx.env);

    ctx.contract.register_creator(
        &creator_due,
        &name(&ctx.env, "Due"),
        &name(&ctx.env, ""),
        &name(&ctx.env, ""),
    );
    ctx.contract.register_creator(
        &creator_not_due,
        &name(&ctx.env, "NotDue"),
        &name(&ctx.env, ""),
        &name(&ctx.env, ""),
    );

    ctx.token_admin.mint(&supporter_due, &10_000_000_000);
    ctx.token_admin.mint(&supporter_not_due, &10_000_000_000);
    ctx.token.approve(
        &supporter_due,
        &ctx.contract.address,
        &10_000_000_000,
        &(SECONDS_PER_MONTH as u32 * 2),
    );
    ctx.token.approve(
        &supporter_not_due,
        &ctx.contract.address,
        &10_000_000_000,
        &(SECONDS_PER_MONTH as u32 * 2),
    );

    // Subscribed first, so by the time we advance one month it is due.
    ctx.contract
        .subscribe(&supporter_due, &creator_due, &20_0000000);

    // Subscribed after the ledger already advanced, so its next_charge_date is
    // a full month further out than the first subscription's.
    ctx.env
        .ledger()
        .with_mut(|li| li.timestamp += SECONDS_PER_MONTH / 2);
    ctx.contract
        .subscribe(&supporter_not_due, &creator_not_due, &30_0000000);

    ctx.env
        .ledger()
        .with_mut(|li| li.timestamp += SECONDS_PER_MONTH / 2);

    let charged = ctx.contract.process_due_subscriptions();

    assert_eq!(charged, 1);
    assert_eq!(ctx.token.balance(&creator_due), 20_0000000);
    assert_eq!(ctx.token.balance(&creator_not_due), 0);

    let not_due_sub = ctx
        .contract
        .get_subscriptions_by_supporter(&supporter_not_due)
        .get(0)
        .unwrap();
    assert!(not_due_sub.next_charge_date > ctx.env.ledger().timestamp());
}

#[test]
fn test_unauthorized_fee_update_rejected() {
    let ctx = setup(0);
    let attacker = Address::generate(&ctx.env);

    let result = ctx.contract.try_update_fee(&attacker, &500);
    assert!(result.is_err());
    assert_eq!(ctx.contract.get_fee(), 0);

    ctx.contract.update_fee(&ctx.admin, &300);
    assert_eq!(ctx.contract.get_fee(), 300);
}
