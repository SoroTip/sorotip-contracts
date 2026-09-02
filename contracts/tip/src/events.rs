use soroban_sdk::{Address, Env, String, Symbol};

/// Publishes a `CreatorRegistered` event when a new creator profile is created.
pub fn creator_registered(
    env: &Env,
    profile_id: u64,
    wallet: &Address,
    name: &String,
    timestamp: u64,
) {
    let topic = (Symbol::new(env, "creator_registered"), wallet.clone());
    env.events()
        .publish(topic, (profile_id, name.clone(), timestamp));
}

/// Publishes a `TipSent` event whenever a tip is transferred.
#[allow(clippy::too_many_arguments)]
pub fn tip_sent(
    env: &Env,
    tip_id: u64,
    from: &Address,
    to: &Address,
    amount: i128,
    fee_paid: i128,
    timestamp: u64,
) {
    let topic = (Symbol::new(env, "tip_sent"), from.clone(), to.clone());
    env.events()
        .publish(topic, (tip_id, amount, fee_paid, timestamp));
}

/// Publishes a `SubscriptionCreated` event when a new subscription is opened.
pub fn subscription_created(
    env: &Env,
    subscription_id: u64,
    supporter: &Address,
    creator: &Address,
    amount_per_month: i128,
    next_charge_date: u64,
) {
    let topic = (
        Symbol::new(env, "sub_created"),
        supporter.clone(),
        creator.clone(),
    );
    env.events()
        .publish(topic, (subscription_id, amount_per_month, next_charge_date));
}

/// Publishes a `SubscriptionCharged` event each time a subscription is billed.
pub fn subscription_charged(
    env: &Env,
    subscription_id: u64,
    supporter: &Address,
    creator: &Address,
    amount: i128,
    timestamp: u64,
) {
    let topic = (
        Symbol::new(env, "sub_charged"),
        supporter.clone(),
        creator.clone(),
    );
    env.events()
        .publish(topic, (subscription_id, amount, timestamp));
}

/// Publishes a `SubscriptionCancelled` event when a supporter cancels.
pub fn subscription_cancelled(
    env: &Env,
    subscription_id: u64,
    supporter: &Address,
    creator: &Address,
    timestamp: u64,
) {
    let topic = (
        Symbol::new(env, "sub_cancelled"),
        supporter.clone(),
        creator.clone(),
    );
    env.events().publish(topic, (subscription_id, timestamp));
}

/// Publishes a `GoalSet` event when a creator publishes a new funding goal.
pub fn goal_set(
    env: &Env,
    goal_id: u64,
    creator: &Address,
    goal_amount: i128,
    description: &String,
) {
    let topic = (Symbol::new(env, "goal_set"), creator.clone());
    env.events()
        .publish(topic, (goal_id, goal_amount, description.clone()));
}

/// Publishes a `GoalReached` event when a funding goal's target has been met.
pub fn goal_reached(env: &Env, goal_id: u64, creator: &Address, goal_amount: i128, timestamp: u64) {
    let topic = (Symbol::new(env, "goal_reached"), creator.clone());
    env.events()
        .publish(topic, (goal_id, goal_amount, timestamp));
}

/// Publishes a `FeeUpdated` event when the admin changes the protocol fee.
pub fn fee_updated(env: &Env, old_fee: u32, new_fee: u32, admin: &Address) {
    let topic = (Symbol::new(env, "fee_updated"), admin.clone());
    env.events().publish(topic, (old_fee, new_fee));
}
