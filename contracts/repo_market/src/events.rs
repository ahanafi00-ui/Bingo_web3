use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug)]
pub struct RepoOpenedEvent {
    pub position_id: u64,
    pub borrower: Address,
    pub series_id: u32,
    pub collateral_par: i128,
    pub cash_out: i128,
    pub repurchase_amount: i128,
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct RepoClosedEvent {
    pub position_id: u64,
    pub borrower: Address,
    pub repayment: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct RepoDefaultedEvent {
    pub position_id: u64,
    pub borrower: Address,
    pub treasury: Address,
    pub collateral_claimed: i128,
}
