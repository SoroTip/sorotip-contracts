//! SoroTip — an on-chain tipping and creator monetization protocol for Stellar Soroban.
//!
//! Tips and subscription charges move funds directly from a supporter's wallet to a
//! creator's wallet in a single token transfer. The contract never custodies funds:
//! it only routes payments and records the resulting history, profiles, and stats.
#![no_std]

mod errors;
mod events;
mod storage;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, panic_with_error, token, Address, Env, String, Vec};

pub use errors::Error;
pub use types::{
    Config, CreatorProfile, LeaderboardEntry, ProtocolStats, Subscription, Tip, TipGoal,
};

use types::{MAX_FEE_BASIS_POINTS, SECONDS_PER_MONTH};

#[contract]
pub struct TipContract;

#[contractimpl]
impl TipContract {
    /// Initializes the contract with an admin, protocol fee, treasury, and the USDC
    /// (or other Stellar Asset Contract) token address that all tips and
    /// subscriptions will be denominated in. Can only be called once.
    pub fn initialize(
        env: Env,
        admin: Address,
        fee_basis_points: u32,
        treasury: Address,
        token: Address,
    ) {
        admin.require_auth();

        if storage::load_config(&env).is_some() {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        if fee_basis_points > MAX_FEE_BASIS_POINTS {
            panic_with_error!(&env, Error::InvalidFee);
        }

        storage::save_config(
            &env,
            &Config {
                admin,
                treasury,
                fee_basis_points,
                token,
            },
        );
    }

    /// Creates a public creator profile for `wallet`. Calling this again for an
    /// already-registered wallet is a no-op that simply returns the existing
    /// profile id — use [`Self::update_profile`] to edit an existing profile.
    pub fn register_creator(
        env: Env,
        wallet: Address,
        name: String,
        bio: String,
        avatar_ipfs: String,
    ) -> u64 {
        wallet.require_auth();
        require_initialized(&env);

        if let Some(existing) = storage::load_profile(&env, &wallet) {
            return existing.id;
        }

        let id = storage::next_profile_id(&env);
        let now = env.ledger().timestamp();
        let profile = CreatorProfile {
            id,
            wallet: wallet.clone(),
            name: name.clone(),
            bio,
            avatar_ipfs,
            total_received: 0,
            tip_count: 0,
            subscriber_count: 0,
            registered_at: now,
        };
        storage::save_profile(&env, &profile);
        storage::update_protocol_stats(&env, |stats| stats.total_creators += 1);
        events::creator_registered(&env, id, &wallet, &name, now);
        id
    }

    /// Updates an existing creator's public profile fields.
    pub fn update_profile(
        env: Env,
        wallet: Address,
        name: String,
        bio: String,
        avatar_ipfs: String,
    ) {
        wallet.require_auth();

        let mut profile = load_profile_or_panic(&env, &wallet);
        profile.name = name;
        profile.bio = bio;
        profile.avatar_ipfs = avatar_ipfs;
        storage::save_profile(&env, &profile);
    }

    /// Sends a one-time tip of `amount` (in the configured token's smallest unit)
    /// directly from `from` to `to`. If a protocol fee is configured, it is
    /// deducted from `amount` and routed to the treasury in the same call.
    pub fn tip(env: Env, from: Address, to: Address, amount: i128, message_ipfs: String) -> u64 {
        from.require_auth();
        let config = require_initialized(&env);
        validate_amount(&env, amount);

        let fee = compute_fee(amount, config.fee_basis_points);
        let net = amount - fee;

        let token_client = token::Client::new(&env, &config.token);
        token_client.transfer(&from, &to, &net);
        if fee > 0 {
            token_client.transfer(&from, &config.treasury, &fee);
        }

        let id = storage::next_tip_id(&env);
        let now = env.ledger().timestamp();
        let tip_record = Tip {
            id,
            from: from.clone(),
            to: to.clone(),
            amount,
            fee_paid: fee,
            message_ipfs,
            timestamp: now,
        };
        storage::save_tip(&env, &tip_record);

        if let Some(mut profile) = storage::load_profile(&env, &to) {
            profile.total_received += net;
            profile.tip_count += 1;
            storage::save_profile(&env, &profile);
            apply_goal_progress(&env, &to, net, now);
        }

        storage::update_protocol_stats(&env, |stats| {
            stats.total_tips += 1;
            stats.total_volume += amount;
            stats.fee_collected += fee;
        });

        events::tip_sent(&env, id, &from, &to, amount, fee, now);
        id
    }

    /// Opens a recurring monthly subscription from `from` to `to`. No funds move
    /// at subscription time — the first charge happens ~30 days later, the next
    /// time anyone calls [`Self::process_due_subscriptions`]. The supporter must
    /// separately approve the contract as a spender on the configured token for
    /// at least `amount_per_month` before each charge is due.
    pub fn subscribe(env: Env, from: Address, to: Address, amount_per_month: i128) -> u64 {
        from.require_auth();
        require_initialized(&env);
        validate_amount(&env, amount_per_month);

        let id = storage::next_subscription_id(&env);
        let now = env.ledger().timestamp();
        let next_charge_date = now + SECONDS_PER_MONTH;
        let subscription = Subscription {
            id,
            supporter: from.clone(),
            creator: to.clone(),
            amount_per_month,
            next_charge_date,
            active: true,
            created_at: now,
        };
        storage::save_subscription(&env, &subscription);
        storage::index_new_subscription(&env, &subscription);

        if let Some(mut profile) = storage::load_profile(&env, &to) {
            profile.subscriber_count += 1;
            storage::save_profile(&env, &profile);
        }

        storage::update_protocol_stats(&env, |stats| stats.total_subscriptions += 1);
        events::subscription_created(&env, id, &from, &to, amount_per_month, next_charge_date);
        id
    }

    /// Cancels a subscription. Only the original supporter may cancel it.
    pub fn cancel_subscription(env: Env, from: Address, subscription_id: u64) {
        from.require_auth();

        let subscription = match storage::load_subscription(&env, subscription_id) {
            Some(s) => s,
            None => panic_with_error!(&env, Error::SubscriptionNotFound),
        };
        if subscription.supporter != from {
            panic_with_error!(&env, Error::NotSubscriber);
        }
        if !subscription.active {
            panic_with_error!(&env, Error::SubscriptionAlreadyCancelled);
        }

        storage::cancel_subscription_storage(&env, subscription_id);

        if let Some(mut profile) = storage::load_profile(&env, &subscription.creator) {
            if profile.subscriber_count > 0 {
                profile.subscriber_count -= 1;
            }
            storage::save_profile(&env, &profile);
        }

        let now = env.ledger().timestamp();
        events::subscription_cancelled(&env, subscription_id, &from, &subscription.creator, now);
    }

    /// Charges every active subscription whose `next_charge_date` has passed.
    /// Callable by anyone (a "keeper"). Subscriptions whose supporter has not
    /// granted enough token allowance to the contract are skipped and remain
    /// due for the next call, rather than failing the whole batch. Returns the
    /// number of subscriptions successfully charged.
    pub fn process_due_subscriptions(env: Env) -> u32 {
        let config = require_initialized(&env);
        let now = env.ledger().timestamp();
        let due = storage::get_due_subscriptions(&env, now);
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &config.token);

        let mut charged: u32 = 0;
        for mut subscription in due.iter() {
            let allowance = token_client.allowance(&subscription.supporter, &contract_address);
            if allowance < subscription.amount_per_month {
                continue;
            }

            let fee = compute_fee(subscription.amount_per_month, config.fee_basis_points);
            let net = subscription.amount_per_month - fee;

            token_client.transfer_from(
                &contract_address,
                &subscription.supporter,
                &subscription.creator,
                &net,
            );
            if fee > 0 {
                token_client.transfer_from(
                    &contract_address,
                    &subscription.supporter,
                    &config.treasury,
                    &fee,
                );
            }

            subscription.next_charge_date += SECONDS_PER_MONTH;
            storage::save_subscription(&env, &subscription);

            if let Some(mut profile) = storage::load_profile(&env, &subscription.creator) {
                profile.total_received += net;
                storage::save_profile(&env, &profile);
                apply_goal_progress(&env, &subscription.creator, net, now);
            }

            storage::update_protocol_stats(&env, |stats| {
                stats.total_volume += subscription.amount_per_month;
                stats.fee_collected += fee;
            });

            events::subscription_charged(
                &env,
                subscription.id,
                &subscription.supporter,
                &subscription.creator,
                subscription.amount_per_month,
                now,
            );
            charged += 1;
        }

        charged
    }

    /// Publishes a new funding goal for `creator`, replacing any previous one.
    pub fn set_tip_goal(env: Env, creator: Address, goal_amount: i128, description: String) -> u64 {
        creator.require_auth();

        if !storage::profile_exists(&env, &creator) {
            panic_with_error!(&env, Error::CreatorNotFound);
        }
        if goal_amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }

        let id = storage::next_goal_id(&env);
        let now = env.ledger().timestamp();
        let goal = TipGoal {
            id,
            creator: creator.clone(),
            goal_amount,
            current_amount: 0,
            description: description.clone(),
            completed: false,
            created_at: now,
        };
        storage::save_goal(&env, &goal);
        events::goal_set(&env, id, &creator, goal_amount, &description);
        id
    }

    /// Manually marks a creator's tip goal as completed. Goals are also
    /// completed automatically inside [`Self::tip`] and
    /// [`Self::process_due_subscriptions`] once `current_amount` reaches
    /// `goal_amount`; this entry point lets a creator close a goal early.
    pub fn complete_tip_goal(env: Env, creator: Address, goal_id: u64) {
        creator.require_auth();

        let mut goal = match storage::load_goal(&env, &creator) {
            Some(g) => g,
            None => panic_with_error!(&env, Error::GoalNotFound),
        };
        if goal.id != goal_id {
            panic_with_error!(&env, Error::GoalNotFound);
        }
        if goal.completed {
            panic_with_error!(&env, Error::GoalAlreadyCompleted);
        }

        goal.completed = true;
        storage::save_goal(&env, &goal);

        let now = env.ledger().timestamp();
        events::goal_reached(&env, goal.id, &creator, goal.goal_amount, now);
    }

    /// Returns a creator's public profile.
    pub fn get_profile(env: Env, wallet: Address) -> CreatorProfile {
        load_profile_or_panic(&env, &wallet)
    }

    /// Returns up to `limit` tips involving `wallet` (as sender or recipient),
    /// most recent first.
    pub fn get_tip_history(env: Env, wallet: Address, limit: u32) -> Vec<Tip> {
        let ids = storage::tip_ids_for_wallet(&env, &wallet);
        let len = ids.len();
        let take = core::cmp::min(limit, len);

        let mut result = Vec::new(&env);
        let mut i = len;
        let mut collected = 0u32;
        while i > 0 && collected < take {
            i -= 1;
            if let Some(t) = storage::load_tip(&env, ids.get(i).unwrap()) {
                result.push_back(t);
                collected += 1;
            }
        }
        result
    }

    /// Returns every subscription a supporter has ever opened.
    pub fn get_subscriptions_by_supporter(env: Env, supporter: Address) -> Vec<Subscription> {
        let ids = storage::subscription_ids_by_supporter(&env, &supporter);
        load_subscriptions(&env, &ids)
    }

    /// Returns every subscription a creator has ever received.
    pub fn get_subscriptions_by_creator(env: Env, creator: Address) -> Vec<Subscription> {
        let ids = storage::subscription_ids_by_creator(&env, &creator);
        load_subscriptions(&env, &ids)
    }

    /// Returns a creator's current tip goal, if one has been set.
    pub fn get_tip_goal(env: Env, creator: Address) -> Option<TipGoal> {
        storage::load_goal(&env, &creator)
    }

    /// Returns aggregate, protocol-wide statistics.
    pub fn get_protocol_stats(env: Env) -> ProtocolStats {
        storage::load_stats(&env)
    }

    /// Returns the top `limit` creators ordered by total USDC received, descending.
    pub fn get_top_creators(env: Env, limit: u32) -> Vec<LeaderboardEntry> {
        let wallets = storage::all_creator_wallets(&env);
        let mut entries: Vec<LeaderboardEntry> = Vec::new(&env);
        for wallet in wallets.iter() {
            if let Some(profile) = storage::load_profile(&env, &wallet) {
                entries.push_back(LeaderboardEntry {
                    wallet: profile.wallet,
                    name: profile.name,
                    total_received: profile.total_received,
                    tip_count: profile.tip_count,
                });
            }
        }

        sort_leaderboard_desc(&mut entries);

        let take = core::cmp::min(limit, entries.len());
        let mut result = Vec::new(&env);
        for i in 0..take {
            result.push_back(entries.get(i).unwrap());
        }
        result
    }

    /// Updates the protocol fee. Only the configured admin may call this.
    pub fn update_fee(env: Env, admin: Address, fee_basis_points: u32) {
        admin.require_auth();

        let mut config = require_initialized(&env);
        if admin != config.admin {
            panic_with_error!(&env, Error::Unauthorized);
        }
        if fee_basis_points > MAX_FEE_BASIS_POINTS {
            panic_with_error!(&env, Error::InvalidFee);
        }

        let old_fee = config.fee_basis_points;
        config.fee_basis_points = fee_basis_points;
        storage::save_config(&env, &config);
        events::fee_updated(&env, old_fee, fee_basis_points, &admin);
    }

    /// Returns the current protocol fee, in basis points.
    pub fn get_fee(env: Env) -> u32 {
        require_initialized(&env).fee_basis_points
    }
}

fn require_initialized(env: &Env) -> Config {
    match storage::load_config(env) {
        Some(config) => config,
        None => panic_with_error!(env, Error::NotInitialized),
    }
}

fn load_profile_or_panic(env: &Env, wallet: &Address) -> CreatorProfile {
    match storage::load_profile(env, wallet) {
        Some(profile) => profile,
        None => panic_with_error!(env, Error::CreatorNotFound),
    }
}

fn validate_amount(env: &Env, amount: i128) {
    if amount == 0 {
        panic_with_error!(env, Error::ZeroAmount);
    }
    if amount < 0 {
        panic_with_error!(env, Error::InvalidAmount);
    }
}

fn compute_fee(amount: i128, fee_basis_points: u32) -> i128 {
    amount * (fee_basis_points as i128) / 10_000
}

fn apply_goal_progress(env: &Env, creator: &Address, amount: i128, now: u64) {
    if let Some(mut goal) = storage::load_goal(env, creator) {
        if !goal.completed {
            goal.current_amount += amount;
            let reached = goal.current_amount >= goal.goal_amount;
            if reached {
                goal.completed = true;
            }
            storage::save_goal(env, &goal);
            if reached {
                events::goal_reached(env, goal.id, creator, goal.goal_amount, now);
            }
        }
    }
}

fn load_subscriptions(env: &Env, ids: &Vec<u64>) -> Vec<Subscription> {
    let mut result = Vec::new(env);
    for id in ids.iter() {
        if let Some(subscription) = storage::load_subscription(env, id) {
            result.push_back(subscription);
        }
    }
    result
}

/// Sorts leaderboard entries by `total_received` descending, in place.
fn sort_leaderboard_desc(entries: &mut Vec<LeaderboardEntry>) {
    let len = entries.len();
    for i in 1..len {
        let current = entries.get(i).unwrap();
        let mut j = i;
        while j > 0 {
            let prev = entries.get(j - 1).unwrap();
            if prev.total_received < current.total_received {
                entries.set(j, prev);
                j -= 1;
            } else {
                break;
            }
        }
        entries.set(j, current);
    }
}
