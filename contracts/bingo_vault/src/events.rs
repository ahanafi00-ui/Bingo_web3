use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug)]
pub struct SeriesCreatedEvent {
    pub series_id: u32,
    pub issue_date: u64,
    pub maturity_date: u64,
    pub issue_price: i128,
    pub cap_par: i128,
    pub user_cap_par: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SeriesActivatedEvent {
    pub series_id: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SubscribedEvent {
    pub series_id: u32,
    pub user: Address,
    pub pay_amount: i128,
    pub minted_par: i128,
    pub price: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct RedeemedEvent {
    pub series_id: u32,
    pub user: Address,
    pub bt_bill_amount: i128,
    pub payout: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SeriesMaturedEvent {
    pub series_id: u32,
}
