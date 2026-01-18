use crate::storage::{PAR_UNIT, Series};

/// Calculate current price with linear accretion
/// 
/// Formula: price(t) = issue_price + (PAR - issue_price) × (elapsed / total_duration)
/// 
/// Example:
/// - issue_price: 0.95, issue_date: Day 0, maturity: Day 365
/// - At Day 182: price = 0.95 + (1.0 - 0.95) × (182/365) = 0.975
pub fn calculate_current_price(series: &Series, current_time: u64) -> i128 {
    // Before issue date, use issue price
    if current_time <= series.issue_date {
        return series.issue_price;
    }

    // At or after maturity, return PAR
    if current_time >= series.maturity_date {
        return PAR_UNIT;
    }

    // Linear interpolation between issue and maturity
    let elapsed = current_time - series.issue_date;
    let total_duration = series.maturity_date - series.issue_date;
    
    let price_delta = PAR_UNIT - series.issue_price;
    let accreted_value = (price_delta as i128)
        .checked_mul(elapsed as i128)
        .and_then(|v| v.checked_div(total_duration as i128))
        .unwrap_or(0);

    series.issue_price + accreted_value
}

/// Calculate how many PAR units to mint for a given payment
/// 
/// Formula: minted_par = pay_amount × PAR_UNIT / current_price
/// 
/// Example:
/// - pay_amount: 9,500 USDC
/// - current_price: 0.95
/// - minted_par: 9,500 × 1.0 / 0.95 = 10,000 PAR
pub fn calculate_minted_par(pay_amount: i128, current_price: i128) -> Option<i128> {
    pay_amount
        .checked_mul(PAR_UNIT)?
        .checked_div(current_price)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Series, SeriesStatus, SCALE};

    #[test]
    fn test_price_at_issue() {
        let series = Series {
            series_id: 1,
            issue_date: 1000,
            maturity_date: 2000,
            par_unit: PAR_UNIT,
            issue_price: 95 * SCALE / 100, // 0.95
            cap_par: 1_000_000 * SCALE,
            minted_par: 0,
            user_cap_par: 100_000 * SCALE,
            status: SeriesStatus::Active,
        };

        let price = calculate_current_price(&series, 1000);
        assert_eq!(price, 95 * SCALE / 100); // 0.95
    }

    #[test]
    fn test_price_at_maturity() {
        let series = Series {
            series_id: 1,
            issue_date: 1000,
            maturity_date: 2000,
            par_unit: PAR_UNIT,
            issue_price: 95 * SCALE / 100,
            cap_par: 1_000_000 * SCALE,
            minted_par: 0,
            user_cap_par: 100_000 * SCALE,
            status: SeriesStatus::Active,
        };

        let price = calculate_current_price(&series, 2000);
        assert_eq!(price, PAR_UNIT); // 1.0
    }

    #[test]
    fn test_price_halfway() {
        let series = Series {
            series_id: 1,
            issue_date: 1000,
            maturity_date: 2000,
            par_unit: PAR_UNIT,
            issue_price: 95 * SCALE / 100, // 0.95
            cap_par: 1_000_000 * SCALE,
            minted_par: 0,
            user_cap_par: 100_000 * SCALE,
            status: SeriesStatus::Active,
        };

        let price = calculate_current_price(&series, 1500); // Halfway
        assert_eq!(price, 975 * SCALE / 1000); // 0.975
    }

    #[test]
    fn test_calculate_minted_par() {
        let pay_amount = 95 * SCALE; // 95 USDC
        let current_price = 95 * SCALE / 100; // 0.95
        
        let minted = calculate_minted_par(pay_amount, current_price).unwrap();
        assert_eq!(minted, 100 * SCALE); // 100 PAR
    }
}
