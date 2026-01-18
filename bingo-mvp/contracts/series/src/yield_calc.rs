use crate::types::{Series, SCALE};
use soroban_sdk::Env;

pub struct YieldCalculator;

impl YieldCalculator {
    /// Calculate current index for a series
    /// Index represents the current price per SCALE shares
    pub fn calculate_index(env: &Env, series: &Series) -> i128 {
        let now = env.ledger().timestamp();

        // If matured, return par index (SCALE = 1.0)
        if now >= series.maturity_time {
            return SCALE;
        }

        // Calculate elapsed time
        let elapsed = (now - series.issue_time) as i128;
        let duration = (series.maturity_time - series.issue_time) as i128;

        // Calculate yield per unit time
        let total_yield = series.par_value - series.subscription_price;

        // Index = subscription_price + (total_yield * elapsed / duration)
        let accrued_yield = (total_yield * elapsed) / duration;
        series.subscription_price + accrued_yield
    }

    /// Calculate shares to mint for given USDC amount
    pub fn calculate_shares(usdc_amount: i128, current_index: i128) -> i128 {
        (usdc_amount * SCALE) / current_index
    }

    /// Calculate position value for given shares
    pub fn calculate_position_value(shares: i128, current_index: i128) -> i128 {
        (shares * current_index) / SCALE
    }

    /// Calculate redemption value at maturity (always par)
    pub fn calculate_redemption_value(shares: i128, par_value: i128) -> i128 {
        (shares * par_value) / SCALE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SeriesStatus;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn create_test_series(env: &Env, issue_time: u64, maturity_time: u64) -> Series {
        Series {
            id: 0,
            par_value: 1_000_000,
            subscription_price: 980_000,
            issue_time,
            maturity_time,
            max_cap: 10_000_000,
            per_user_cap: 1_000_000,
            total_subscribed: 0,
            status: SeriesStatus::Active,
            usdc_token: Address::generate(env),
        }
    }

    #[test]
    fn test_index_at_issue() {
        let env = Env::default();
        let issue_time = env.ledger().timestamp();
        let maturity_time = issue_time + 90 * 24 * 3600;
        
        let series = create_test_series(&env, issue_time, maturity_time);
        let index = YieldCalculator::calculate_index(&env, &series);
        
        assert_eq!(index, series.subscription_price);
    }

    #[test]
    fn test_index_at_maturity() {
        let env = Env::default();
        let issue_time = 1000u64;
        let maturity_time = issue_time + 90 * 24 * 3600;
        
        env.ledger().with_mut(|li| {
            li.timestamp = maturity_time;
        });
        
        let series = create_test_series(&env, issue_time, maturity_time);
        let index = YieldCalculator::calculate_index(&env, &series);
        
        assert_eq!(index, SCALE);
    }

    #[test]
    fn test_index_halfway() {
        let env = Env::default();
        let issue_time = 1000u64;
        let duration = 90 * 24 * 3600u64;
        let maturity_time = issue_time + duration;
        let halfway = issue_time + duration / 2;
        
        env.ledger().with_mut(|li| {
            li.timestamp = halfway;
        });
        
        let series = create_test_series(&env, issue_time, maturity_time);
        let index = YieldCalculator::calculate_index(&env, &series);
        
        let expected = series.subscription_price + (series.par_value - series.subscription_price) / 2;
        assert_eq!(index, expected);
    }

    #[test]
    fn test_calculate_shares() {
        let usdc_amount = 980_000i128;
        let current_index = 980_000i128;
        
        let shares = YieldCalculator::calculate_shares(usdc_amount, current_index);
        
        // Should get SCALE shares (10_000_000)
        assert_eq!(shares, SCALE);
    }

    #[test]
    fn test_calculate_position_value() {
        let shares = SCALE;
        let current_index = 990_000i128;
        
        let value = YieldCalculator::calculate_position_value(shares, current_index);
        
        assert_eq!(value, 990_000);
    }
}
