use crate::types::{Series, SeriesStatus};
use soroban_sdk::Env;

pub struct Validator;

impl Validator {
    pub fn validate_series_params(
        env: &Env,
        par_value: i128,
        subscription_price: i128,
        maturity_time: u64,
        max_cap: i128,
        per_user_cap: i128,
    ) {
        if subscription_price >= par_value {
            panic!("Subscription price must be less than par");
        }
        
        if maturity_time <= env.ledger().timestamp() {
            panic!("Maturity must be in the future");
        }
        
        if max_cap <= 0 {
            panic!("Max cap must be positive");
        }
        
        if per_user_cap <= 0 {
            panic!("Per user cap must be positive");
        }
        
        if per_user_cap > max_cap {
            panic!("Per user cap cannot exceed max cap");
        }
    }

    pub fn validate_subscription(
        env: &Env,
        series: &Series,
        new_shares: i128,
        user_total_shares: i128,
    ) {
        // Check series is active
        if series.status != SeriesStatus::Active {
            panic!("Series not active");
        }

        // Check not matured
        if env.ledger().timestamp() >= series.maturity_time {
            panic!("Series has matured");
        }

        // Check max cap
        if series.total_subscribed + new_shares > series.max_cap {
            panic!("Exceeds max cap");
        }

        // Check per-user cap
        if user_total_shares > series.per_user_cap {
            panic!("Exceeds per-user cap");
        }
    }

    pub fn validate_redemption(env: &Env, series: &Series) {
        if env.ledger().timestamp() < series.maturity_time {
            panic!("Series not yet matured");
        }
    }

    pub fn validate_settlement(env: &Env, series: &Series, usdc_amount: i128, required: i128) {
        if env.ledger().timestamp() < series.maturity_time {
            panic!("Series not yet matured");
        }
        
        if usdc_amount < required {
            panic!("Insufficient settlement amount");
        }
    }
}
