use soroban_sdk::{contracttype, Address};

/// Represents a single price feed submission.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceSubmission {
    /// Unique identifier for this submission.
    pub id: u64,
    /// The submitter address (must be authorised).
    pub submitter: Address,
    /// Feed identifier (e.g., `"BTC/USD"`, `"ETH/USD"`).
    pub feed_id: soroban_sdk::Symbol,
    /// The price value (fixed-point; e.g., 50000 × 10^8 for $50,000 BTC).
    pub price: i128,
    /// Number of decimal places in the price (typically 8 for USD pairs).
    pub decimals: u32,
    /// Timestamp when the price was observed off-chain.
    pub timestamp: u64,
    /// Nonce for replay protection (incremented per submitter per feed).
    pub nonce: u64,
}

/// Represents the latest value for a feed with metadata.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedLatest {
    /// Feed identifier.
    pub feed_id: soroban_sdk::Symbol,
    /// Latest price.
    pub price: i128,
    /// Decimal places.
    pub decimals: u32,
    /// Timestamp of the latest submission.
    pub timestamp: u64,
    /// Number of submissions for this feed.
    pub submission_count: u64,
}

/// Represents a registered submitter with permissions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubmitterInfo {
    /// Submitter address.
    pub address: Address,
    /// Whether this submitter is currently active.
    pub active: bool,
    /// When the submitter was registered (timestamp).
    pub registered_at: u64,
}
