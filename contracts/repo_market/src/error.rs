use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    // ============================================
    // INITIALIZATION ERRORS (1-5)
    // ============================================
    /// Contract already initialized
    AlreadyInitialized = 1,
    /// Contract not initialized
    NotInitialized = 2,
    
    // ============================================
    // AUTHORIZATION ERRORS (10-15)
    // ============================================
    /// Caller not authorized (not treasury)
    Unauthorized = 10,
    
    // ============================================
    // POSITION ERRORS (20-29)
    // ============================================
    /// Repo position not found
    PositionNotFound = 20,
    /// Invalid position status for this operation
    InvalidStatus = 21,
    
    // ============================================
    // AMOUNT ERRORS (30-39)
    // ============================================
    /// Amount must be positive
    InvalidAmount = 30,
    /// Requested cash exceeds LTV limit (collateral × price × (1 - haircut))
    ExceedsMaxCash = 31,
    
    // ============================================
    // DEADLINE ERRORS (40-49)
    // ============================================
    /// Deadline must be ≤ series maturity date
    InvalidDeadline = 40,
    /// Cannot claim default: deadline not yet passed
    DeadlineNotPassed = 41,
    /// Cannot close repo: deadline already passed (defaulted)
    DeadlinePassed = 42,
    
    // ============================================
    // OPERATIONAL ERRORS (50-59)
    // ============================================
    /// Contract is paused
    ContractPaused = 50,
}
