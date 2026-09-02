use soroban_sdk::contracterror;

/// All error conditions that can be returned by the SoroTip contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The contract has not been initialized yet.
    NotInitialized = 1,
    /// The contract has already been initialized.
    AlreadyInitialized = 2,
    /// No creator profile exists for the given wallet address.
    CreatorNotFound = 3,
    /// The caller is not the creator associated with this resource.
    NotCreator = 4,
    /// No subscription exists for the given subscription id.
    SubscriptionNotFound = 5,
    /// The caller is not the supporter who owns this subscription.
    NotSubscriber = 6,
    /// The subscription has already been cancelled.
    SubscriptionAlreadyCancelled = 7,
    /// The requested fee is outside the allowed range (0-500 basis points).
    InvalidFee = 8,
    /// The requested amount is zero, which is not a valid tip or subscription amount.
    ZeroAmount = 9,
    /// The caller has not authorized enough of the asset to complete the transfer.
    InsufficientAllowance = 10,
    /// No tip goal exists for the given creator.
    GoalNotFound = 11,
    /// The tip goal has already been marked as completed.
    GoalAlreadyCompleted = 12,
    /// The provided amount is invalid (e.g. negative).
    InvalidAmount = 13,
    /// The caller is not authorized to perform this admin-only action.
    Unauthorized = 14,
}
