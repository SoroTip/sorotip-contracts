use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::types::{Config, CreatorProfile, ProtocolStats, Subscription, Tip, TipGoal};

/// Approximate number of 5-second ledgers in one day, used for TTL bumps.
const DAY_IN_LEDGERS: u32 = 17_280;
/// How far in advance of expiry persistent entries get their TTL extended.
const BUMP_THRESHOLD: u32 = DAY_IN_LEDGERS * 7;
/// How far into the future persistent entries get their TTL extended to.
const BUMP_AMOUNT: u32 = DAY_IN_LEDGERS * 60;

/// All keys used in contract storage.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Config,
    Stats,
    ProfileIdCounter,
    TipIdCounter,
    SubscriptionIdCounter,
    GoalIdCounter,
    Profile(Address),
    CreatorList,
    Tip(u64),
    TipsByWallet(Address),
    Subscription(u64),
    SubscriptionIds,
    SubsBySupporter(Address),
    SubsByCreator(Address),
    Goal(Address),
}

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(BUMP_THRESHOLD, BUMP_AMOUNT);
}

fn bump_persistent(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, BUMP_THRESHOLD, BUMP_AMOUNT);
}

// ---------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------

/// Persists the global contract configuration.
pub fn save_config(env: &Env, config: &Config) {
    env.storage().instance().set(&DataKey::Config, config);
    bump_instance(env);
}

/// Loads the global contract configuration, if the contract has been initialized.
pub fn load_config(env: &Env) -> Option<Config> {
    env.storage().instance().get(&DataKey::Config)
}

// ---------------------------------------------------------------------
// Id counters
// ---------------------------------------------------------------------

fn next_id(env: &Env, key: DataKey) -> u64 {
    let current: u64 = env.storage().instance().get(&key).unwrap_or(0);
    let next = current + 1;
    env.storage().instance().set(&key, &next);
    bump_instance(env);
    next
}

/// Allocates and returns the next creator profile id.
pub fn next_profile_id(env: &Env) -> u64 {
    next_id(env, DataKey::ProfileIdCounter)
}

/// Allocates and returns the next tip id.
pub fn next_tip_id(env: &Env) -> u64 {
    next_id(env, DataKey::TipIdCounter)
}

/// Allocates and returns the next subscription id.
pub fn next_subscription_id(env: &Env) -> u64 {
    next_id(env, DataKey::SubscriptionIdCounter)
}

/// Allocates and returns the next tip goal id.
pub fn next_goal_id(env: &Env) -> u64 {
    next_id(env, DataKey::GoalIdCounter)
}

// ---------------------------------------------------------------------
// Creator profiles
// ---------------------------------------------------------------------

/// Persists a creator profile, keyed by wallet address, and indexes it for the leaderboard.
pub fn save_profile(env: &Env, profile: &CreatorProfile) {
    let key = DataKey::Profile(profile.wallet.clone());
    let is_new = !env.storage().persistent().has(&key);
    env.storage().persistent().set(&key, profile);
    bump_persistent(env, &key);

    if is_new {
        let list_key = DataKey::CreatorList;
        let mut creators: Vec<Address> = env
            .storage()
            .persistent()
            .get(&list_key)
            .unwrap_or_else(|| Vec::new(env));
        creators.push_back(profile.wallet.clone());
        env.storage().persistent().set(&list_key, &creators);
        bump_persistent(env, &list_key);
    }
}

/// Loads a creator profile by wallet address.
pub fn load_profile(env: &Env, wallet: &Address) -> Option<CreatorProfile> {
    env.storage()
        .persistent()
        .get(&DataKey::Profile(wallet.clone()))
}

/// Returns whether a creator profile already exists for the given wallet.
pub fn profile_exists(env: &Env, wallet: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Profile(wallet.clone()))
}

/// Returns every wallet address that has ever registered a creator profile.
pub fn all_creator_wallets(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::CreatorList)
        .unwrap_or_else(|| Vec::new(env))
}

// ---------------------------------------------------------------------
// Tips
// ---------------------------------------------------------------------

/// Persists a tip record and indexes it under both the sender's and recipient's wallets.
pub fn save_tip(env: &Env, tip: &Tip) {
    let key = DataKey::Tip(tip.id);
    env.storage().persistent().set(&key, tip);
    bump_persistent(env, &key);

    add_to_wallet_index(env, DataKey::TipsByWallet(tip.from.clone()), tip.id);
    if tip.to != tip.from {
        add_to_wallet_index(env, DataKey::TipsByWallet(tip.to.clone()), tip.id);
    }
}

/// Loads a tip record by id.
pub fn load_tip(env: &Env, id: u64) -> Option<Tip> {
    env.storage().persistent().get(&DataKey::Tip(id))
}

/// Returns the ids of every tip involving the given wallet, oldest first.
pub fn tip_ids_for_wallet(env: &Env, wallet: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::TipsByWallet(wallet.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

fn add_to_wallet_index(env: &Env, key: DataKey, id: u64) {
    let mut ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env));
    ids.push_back(id);
    env.storage().persistent().set(&key, &ids);
    bump_persistent(env, &key);
}

// ---------------------------------------------------------------------
// Subscriptions
// ---------------------------------------------------------------------

/// Persists a subscription record.
pub fn save_subscription(env: &Env, subscription: &Subscription) {
    let key = DataKey::Subscription(subscription.id);
    env.storage().persistent().set(&key, subscription);
    bump_persistent(env, &key);
}

/// Loads a subscription record by id.
pub fn load_subscription(env: &Env, id: u64) -> Option<Subscription> {
    env.storage().persistent().get(&DataKey::Subscription(id))
}

/// Registers a brand-new subscription id in the supporter, creator, and global indexes.
pub fn index_new_subscription(env: &Env, subscription: &Subscription) {
    add_to_wallet_index(
        env,
        DataKey::SubsBySupporter(subscription.supporter.clone()),
        subscription.id,
    );
    add_to_wallet_index(
        env,
        DataKey::SubsByCreator(subscription.creator.clone()),
        subscription.id,
    );
    add_to_wallet_index(env, DataKey::SubscriptionIds, subscription.id);
}

/// Marks a subscription inactive (cancelled) and persists the change.
pub fn cancel_subscription_storage(env: &Env, id: u64) -> Option<Subscription> {
    let mut subscription: Subscription =
        env.storage().persistent().get(&DataKey::Subscription(id))?;
    subscription.active = false;
    save_subscription(env, &subscription);
    Some(subscription)
}

/// Returns the subscription ids belonging to a supporter.
pub fn subscription_ids_by_supporter(env: &Env, supporter: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::SubsBySupporter(supporter.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

/// Returns the subscription ids belonging to a creator.
pub fn subscription_ids_by_creator(env: &Env, creator: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::SubsByCreator(creator.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

/// Returns every active subscription whose `next_charge_date` has passed.
pub fn get_due_subscriptions(env: &Env, now: u64) -> Vec<Subscription> {
    let all_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::SubscriptionIds)
        .unwrap_or_else(|| Vec::new(env));

    let mut due = Vec::new(env);
    for id in all_ids.iter() {
        if let Some(subscription) = load_subscription(env, id) {
            if subscription.active && subscription.next_charge_date <= now {
                due.push_back(subscription);
            }
        }
    }
    due
}

// ---------------------------------------------------------------------
// Tip goals
// ---------------------------------------------------------------------

/// Persists a creator's current tip goal.
pub fn save_goal(env: &Env, goal: &TipGoal) {
    let key = DataKey::Goal(goal.creator.clone());
    env.storage().persistent().set(&key, goal);
    bump_persistent(env, &key);
}

/// Loads a creator's current tip goal, if one has been set.
pub fn load_goal(env: &Env, creator: &Address) -> Option<TipGoal> {
    env.storage()
        .persistent()
        .get(&DataKey::Goal(creator.clone()))
}

// ---------------------------------------------------------------------
// Protocol stats
// ---------------------------------------------------------------------

/// Loads the current protocol-wide statistics, defaulting to all zeros.
pub fn load_stats(env: &Env) -> ProtocolStats {
    env.storage()
        .instance()
        .get(&DataKey::Stats)
        .unwrap_or(ProtocolStats {
            total_tips: 0,
            total_volume: 0,
            total_creators: 0,
            total_subscriptions: 0,
            fee_collected: 0,
        })
}

/// Loads the current protocol stats, applies `mutate`, and persists the result.
pub fn update_protocol_stats(env: &Env, mutate: impl FnOnce(&mut ProtocolStats)) {
    let mut stats = load_stats(env);
    mutate(&mut stats);
    env.storage().instance().set(&DataKey::Stats, &stats);
    bump_instance(env);
}
