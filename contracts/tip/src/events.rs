use soroban_sdk::{contractevent, Address, Env, String};

#[contractevent(topics = ["creator_registered"])]
pub struct CreatorRegistered {
    #[topic]
    pub wallet: Address,
    pub profile_id: u64,
    pub name: String,
    pub timestamp: u64,
}

#[contractevent(topics = ["tip_sent"])]
pub struct TipSent {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub tip_id: u64,
    pub amount: i128,
    pub fee_paid: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["sub_created"])]
pub struct SubscriptionCreated {
    #[topic]
    pub supporter: Address,
    #[topic]
    pub creator: Address,
    pub subscription_id: u64,
    pub amount_per_month: i128,
    pub next_charge_date: u64,
}

#[contractevent(topics = ["sub_charged"])]
pub struct SubscriptionCharged {
    #[topic]
    pub supporter: Address,
    #[topic]
    pub creator: Address,
    pub subscription_id: u64,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["sub_cancelled"])]
pub struct SubscriptionCancelled {
    #[topic]
    pub supporter: Address,
    #[topic]
    pub creator: Address,
    pub subscription_id: u64,
    pub timestamp: u64,
}

#[contractevent(topics = ["goal_set"])]
pub struct GoalSet {
    #[topic]
    pub creator: Address,
    pub goal_id: u64,
    pub goal_amount: i128,
    pub description: String,
}

#[contractevent(topics = ["goal_reached"])]
pub struct GoalReached {
    #[topic]
    pub creator: Address,
    pub goal_id: u64,
    pub goal_amount: i128,
    pub timestamp: u64,
}

#[contractevent(topics = ["fee_updated"])]
pub struct FeeUpdated {
    #[topic]
    pub admin: Address,
    pub old_fee: u32,
    pub new_fee: u32,
}

/// Publishes a `CreatorRegistered` event when a new creator profile is created.
pub fn creator_registered(
    env: &Env,
    profile_id: u64,
    wallet: &Address,
    name: &String,
    timestamp: u64,
) {
    CreatorRegistered {
        wallet: wallet.clone(),
        profile_id,
        name: name.clone(),
        timestamp,
    }
    .publish(env);
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
    TipSent {
        from: from.clone(),
        to: to.clone(),
        tip_id,
        amount,
        fee_paid,
        timestamp,
    }
    .publish(env);
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
    SubscriptionCreated {
        supporter: supporter.clone(),
        creator: creator.clone(),
        subscription_id,
        amount_per_month,
        next_charge_date,
    }
    .publish(env);
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
    SubscriptionCharged {
        supporter: supporter.clone(),
        creator: creator.clone(),
        subscription_id,
        amount,
        timestamp,
    }
    .publish(env);
}

/// Publishes a `SubscriptionCancelled` event when a supporter cancels.
pub fn subscription_cancelled(
    env: &Env,
    subscription_id: u64,
    supporter: &Address,
    creator: &Address,
    timestamp: u64,
) {
    SubscriptionCancelled {
        supporter: supporter.clone(),
        creator: creator.clone(),
        subscription_id,
        timestamp,
    }
    .publish(env);
}

/// Publishes a `GoalSet` event when a creator publishes a new funding goal.
pub fn goal_set(
    env: &Env,
    goal_id: u64,
    creator: &Address,
    goal_amount: i128,
    description: &String,
) {
    GoalSet {
        creator: creator.clone(),
        goal_id,
        goal_amount,
        description: description.clone(),
    }
    .publish(env);
}

/// Publishes a `GoalReached` event when a funding goal's target has been met.
pub fn goal_reached(env: &Env, goal_id: u64, creator: &Address, goal_amount: i128, timestamp: u64) {
    GoalReached {
        creator: creator.clone(),
        goal_id,
        goal_amount,
        timestamp,
    }
    .publish(env);
}

/// Publishes a `FeeUpdated` event when the admin changes the protocol fee.
pub fn fee_updated(env: &Env, old_fee: u32, new_fee: u32, admin: &Address) {
    FeeUpdated {
        admin: admin.clone(),
        old_fee,
        new_fee,
    }
    .publish(env);
}
