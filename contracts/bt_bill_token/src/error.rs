use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    // Initialization errors
    AlreadyInitialized = 1,
    NotInitialized = 2,
    
    // Authorization errors
    Unauthorized = 3,
    NotOperator = 6,
    
    // Balance errors
    InsufficientBalance = 4,
    InvalidAmount = 5,
}
