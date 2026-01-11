use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Series {
    pub id: u32,
    pub par_value: i128,              // Face value (e.g., 1_000_000 for $1)
    pub subscription_price: i128,     // Initial discount price
    pub issue_time: u64,              // Timestamp when issued
    pub maturity_time: u64,           // Maturity timestamp
    pub max_cap: i128,                // Maximum total subscription cap
    pub per_user_cap: i128,           // Maximum per user
    pub total_subscribed: i128,       // Current total subscribed
    pub status: SeriesStatus,
    pub usdc_token: Address,          // USDC token address for payments
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SeriesStatus {
    Active,
    Matured,
    Settled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserPosition {
    pub shares: i128,                 // Number of shares owned
    pub entry_index: i128,            // Index at subscription (scaled by 1e7)
}

#[contracttype]
pub enum DataKey {
    Admin,
    NextSeriesId,
    Series(u32),                      // series_id -> Series
    UserPosition(u32, Address),       // (series_id, user) -> UserPosition
    KYCVerified(Address),             // user -> bool
}

pub const SCALE: i128 = 10_000_000; // 1e7 for precision
