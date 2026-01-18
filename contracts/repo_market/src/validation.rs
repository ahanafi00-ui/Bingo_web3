use crate::storage::BASIS_POINTS;

/// Calculate maximum cash that can be borrowed
///
/// Formula: max_cash = collateral_value × (1 - haircut)
///
/// Example:
/// - collateral: 10,000 PAR
/// - mark_price: 0.99
/// - haircut: 3% (300 basis points)
/// - collateral_value: 10,000 × 0.99 = 9,900
/// - max_cash: 9,900 × 97% = 9,603
pub fn calculate_max_cash(collateral_par: i128, mark_price: i128, haircut_bps: i128) -> Option<i128> {
    let collateral_value = collateral_par.checked_mul(mark_price)?.checked_div(10_000_000)?; // Divide by SCALE

    let ltv_bps = BASIS_POINTS.checked_sub(haircut_bps)?; // 10,000 - 300 = 9,700 (97%)
    
    collateral_value.checked_mul(ltv_bps)?.checked_div(BASIS_POINTS)
}

/// Calculate repurchase amount
///
/// Formula: repurchase = cash_out × (1 + spread)
///
/// Example:
/// - cash_out: 9,000
/// - spread: 2% (200 basis points)
/// - repurchase: 9,000 × 102% = 9,180
pub fn calculate_repurchase(cash_out: i128, spread_bps: i128) -> Option<i128> {
    let multiplier = BASIS_POINTS.checked_add(spread_bps)?; // 10,000 + 200 = 10,200
    
    cash_out.checked_mul(multiplier)?.checked_div(BASIS_POINTS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_max_cash() {
        let collateral_par = 10_000 * 10_000_000; // 10,000 PAR
        let mark_price = 99 * 10_000_000 / 100; // 0.99
        let haircut_bps = 300; // 3%

        let max_cash = calculate_max_cash(collateral_par, mark_price, haircut_bps).unwrap();
        
        // Expected: 10,000 × 0.99 × 97% = 9,603
        assert_eq!(max_cash, 9603 * 10_000_000);
    }

    #[test]
    fn test_calculate_repurchase() {
        let cash_out = 9_000 * 10_000_000; // 9,000
        let spread_bps = 200; // 2%

        let repurchase = calculate_repurchase(cash_out, spread_bps).unwrap();
        
        // Expected: 9,000 × 102% = 9,180
        assert_eq!(repurchase, 9180 * 10_000_000);
    }

    #[test]
    fn test_zero_haircut() {
        let collateral_par = 10_000 * 10_000_000;
        let mark_price = 10_000_000; // 1.0
        let haircut_bps = 0; // 0%

        let max_cash = calculate_max_cash(collateral_par, mark_price, haircut_bps).unwrap();
        
        // Expected: 10,000 × 1.0 × 100% = 10,000
        assert_eq!(max_cash, 10_000 * 10_000_000);
    }

    #[test]
    fn test_high_haircut() {
        let collateral_par = 10_000 * 10_000_000;
        let mark_price = 10_000_000; // 1.0
        let haircut_bps = 5000; // 50%

        let max_cash = calculate_max_cash(collateral_par, mark_price, haircut_bps).unwrap();
        
        // Expected: 10,000 × 1.0 × 50% = 5,000
        assert_eq!(max_cash, 5_000 * 10_000_000);
    }
}
