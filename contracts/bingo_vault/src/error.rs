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
    /// Caller not authorized (not admin/treasury)
    Unauthorized = 10,
    
    // ============================================
    // SERIES MANAGEMENT ERRORS (20-29)
    // ============================================
    /// Series not found
    SeriesNotFound = 20,
    /// Series already exists with this ID
    SeriesAlreadyExists = 21,
    /// Series not in ACTIVE status
    SeriesNotActive = 22,
    /// Series not yet matured
    SeriesNotMatured = 23,
    /// Invalid series status transition
    InvalidStatus = 24,
    
    // ============================================
    // SUBSCRIPTION ERRORS (30-39)
    // ============================================
    /// Subscription would exceed series cap_par
    ExceedsSeriesCap = 30,
    /// Subscription would exceed user's cap_par
    ExceedsUserCap = 31,
    
    // ============================================
    // AMOUNT/BALANCE ERRORS (40-49)
    // ============================================
    /// Amount must be positive
    InvalidAmount = 40,
    /// User doesn't have enough bT-Bills
    InsufficientBalance = 41,
    
    // ============================================
    // TIMESTAMP/VALIDATION ERRORS (50-59)
    // ============================================
    /// Maturity date must be after issue date
    InvalidTimestamp = 50,
    /// Issue price must be between 0 and PAR_UNIT
    InvalidIssuePrice = 51,
    /// Cap amounts must be positive and user_cap <= series_cap
    InvalidCapAmounts = 52,
    
    // ============================================
    // OPERATIONAL ERRORS (60-69)
    // ============================================
    /// Contract is paused
    ContractPaused = 60,
}
