use soroban_sdk::{contracttype, Address};

// Constants
pub const SCALE: i128 = 10_000_000; // 7 decimals
pub const PAR_UNIT: i128 = 1 * SCALE; // 1.0000000

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SeriesStatus {
    /// Series created but not yet active for subscriptions
    Upcoming = 0,
    /// Series is active and users can subscribe
    Active = 1,
    /// Series has reached maturity date, redemptions allowed
    Matured = 2,
    /// Series ended (optional final state)
    Closed = 3,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Series {
    /// Unique series identifier
    pub series_id: u32,
    /// Unix timestamp when series starts
    pub issue_date: u64,
    /// Unix timestamp when series matures
    pub maturity_date: u64,
    /// PAR unit value (always 1.0 in scaled format)
    pub par_unit: i128,
    /// Initial discounted price (e.g. 0.95)
    pub issue_price: i128,
    /// Maximum PAR units that can be minted for this series
    pub cap_par: i128,
    /// Current PAR units minted
    pub minted_par: i128,
    /// Maximum PAR units per user (anti-monopoly)
    pub user_cap_par: i128,
    /// Current series status
    pub status: SeriesStatus,
    /// Total USDC collected from subscriptions (for accounting)
    pub total_subscriptions_collected: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct UserPosition {
    /// Total PAR units subscribed by this user in this series
    pub subscribed_par: i128,
}

/// Protocol-wide accounting for revenue tracking
/// 
/// With 100% liquidity model:
/// - ALL subscription USDC available for repo lending
/// - No pre-reserved amounts
/// - Haircut ensures safety
/// - Profit calculated at maturity
#[contracttype]
#[derive(Clone, Debug)]
pub struct ProtocolAccounting {
    /// Total USDC collected from all subscriptions
    pub total_subscriptions_collected: i128,
    /// Total PAR units minted (redemption liability at maturity)
    pub total_par_minted: i128,
    /// USDC currently lent out via repo market
    pub total_lent: i128,
    /// USDC received from repo repayments (principal + spread)
    pub total_repo_revenue: i128,
    /// Number of defaults (for analytics)
    pub total_defaults: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Treasury,
    Stablecoin,
    BTBillToken,
    Series(u32),
    UserPosition(u32, Address), // (series_id, user)
    ProtocolAccounting,         // NEW: Global accounting
    Initialized,
    Paused,
}
