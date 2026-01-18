use soroban_sdk::{contracttype, Address};

// Constants
pub const BASIS_POINTS: i128 = 10_000; // 100% = 10,000 basis points

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepoStatus {
    /// Position is open, borrower can still repay
    Open = 0,
    /// Position closed successfully (borrower repaid)
    Closed = 1,
    /// Position defaulted (lender claimed collateral)
    Defaulted = 2,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct RepoPosition {
    /// Unique position ID
    pub id: u64,
    /// Borrower address
    pub borrower: Address,
    /// Series ID of the collateral
    pub series_id: u32,
    /// Collateral amount in PAR units
    pub collateral_par: i128,
    /// Cash given to borrower
    pub cash_out: i128,
    /// Amount borrower must repay (cash_out × (1 + spread))
    pub repurchase_amount: i128,
    /// Timestamp when repo was opened
    pub start_time: u64,
    /// Deadline for repayment
    pub deadline: u64,
    /// Current position status
    pub status: RepoStatus,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Treasury,
    Vault,
    BTBillToken,
    Stablecoin,
    Haircut,      // In basis points (e.g., 300 = 3%)
    Spread,       // In basis points (e.g., 200 = 2%)
    Position(u64), // Position ID → RepoPosition
    PositionCounter,
    Initialized,
    Paused,
}
