use soroban_sdk::{contracttype, Address, String};

/// The number of ledgers a subscription advances by on each successful charge (~30 days).
pub const SECONDS_PER_MONTH: u64 = 30 * 24 * 60 * 60;

/// Maximum protocol fee, expressed in basis points (500 = 5%).
pub const MAX_FEE_BASIS_POINTS: u32 = 500;

/// Global contract configuration set at initialization and updatable by the admin.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub admin: Address,
    pub treasury: Address,
    pub fee_basis_points: u32,
    pub token: Address,
}

/// A creator's public on-chain profile.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorProfile {
    pub id: u64,
    pub wallet: Address,
    pub name: String,
    pub bio: String,
    pub avatar_ipfs: String,
    pub total_received: i128,
    pub tip_count: u32,
    pub subscriber_count: u32,
    pub registered_at: u64,
}

/// A single tip transfer record.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tip {
    pub id: u64,
    pub from: Address,
    pub to: Address,
    pub amount: i128,
    pub fee_paid: i128,
    pub message_ipfs: String,
    pub timestamp: u64,
}

/// A recurring monthly subscription from a supporter to a creator.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Subscription {
    pub id: u64,
    pub supporter: Address,
    pub creator: Address,
    pub amount_per_month: i128,
    pub next_charge_date: u64,
    pub active: bool,
    pub created_at: u64,
}

/// A funding goal a creator publishes on their profile.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TipGoal {
    pub id: u64,
    pub creator: Address,
    pub goal_amount: i128,
    pub current_amount: i128,
    pub description: String,
    pub completed: bool,
    pub created_at: u64,
}

/// Aggregate, protocol-wide statistics.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolStats {
    pub total_tips: u32,
    pub total_volume: i128,
    pub total_creators: u32,
    pub total_subscriptions: u32,
    pub fee_collected: i128,
}

/// A single row in the top-creators leaderboard.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaderboardEntry {
    pub wallet: Address,
    pub name: String,
    pub total_received: i128,
    pub tip_count: u32,
}
