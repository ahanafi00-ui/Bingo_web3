#![no_std]

mod admin;
mod storage;
mod types;
mod user_ops;
mod validation;
mod yield_calc;

use admin::Admin;
use storage::Storage;
use types::{Series, UserPosition, SCALE};
use user_ops::UserOps;

use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct BingoSeries;

#[contractimpl]
impl BingoSeries {
    // ============================================
    // Admin Functions
    // ============================================

    /// Initialize contract with admin
    pub fn initialize(env: Env, admin: Address) {
        Admin::initialize(&env, &admin);
    }

    /// Issue a new obligation series (Admin only)
    pub fn issue_series(
        env: Env,
        par_value: i128,
        subscription_price: i128,
        maturity_time: u64,
        max_cap: i128,
        per_user_cap: i128,
        usdc_token: Address,
    ) -> u32 {
        Admin::issue_series(
            &env,
            par_value,
            subscription_price,
            maturity_time,
            max_cap,
            per_user_cap,
            &usdc_token,
        )
    }

    /// Verify user KYC (Admin only)
    pub fn verify_kyc(env: Env, user: Address) {
        Admin::verify_kyc(&env, &user);
    }

    /// Revoke user KYC (Admin only)
    pub fn revoke_kyc(env: Env, user: Address) {
        Admin::revoke_kyc(&env, &user);
    }

    /// Settle matured series (Admin only)
    pub fn settle_series(env: Env, series_id: u32, usdc_amount: i128, admin: Address) {
        Admin::settle_series(&env, series_id, usdc_amount, &admin);
    }

    // ============================================
    // User Functions
    // ============================================

    /// Subscribe to a series (KYC verified users only)
    pub fn subscribe(env: Env, series_id: u32, usdc_amount: i128, user: Address) -> i128 {
        UserOps::subscribe(&env, series_id, usdc_amount, &user)
    }

    /// Redeem at maturity (KYC verified users only)
    pub fn redeem(env: Env, series_id: u32, user: Address) -> i128 {
        UserOps::redeem(&env, series_id, &user)
    }

    /// Get current value of user's position
    pub fn get_position_value(env: Env, series_id: u32, user: Address) -> i128 {
        UserOps::get_position_value(&env, series_id, &user)
    }

    // ============================================
    // View Functions
    // ============================================

    /// Get series info
    pub fn get_series(env: Env, series_id: u32) -> Series {
        Storage::get_series(&env, series_id)
    }

    /// Get user position
    pub fn get_user_position(env: Env, series_id: u32, user: Address) -> UserPosition {
        Storage::get_user_position(&env, series_id, &user).unwrap_or(UserPosition {
            shares: 0,
            entry_index: SCALE,
        })
    }

    /// Check if user is KYC verified
    pub fn is_kyc_verified(env: Env, user: Address) -> bool {
        Storage::is_kyc_verified(&env, &user)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::Env;

    #[test]
    fn test_full_lifecycle() {
        let env = Env::default();
        env.mock_all_auths();

        // Setup
        let contract_id = env.register_contract(None, BingoSeries);
        let client = BingoSeriesClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let usdc_token = Address::generate(&env);

        // Initialize
        client.initialize(&admin);

        // Verify user KYC
        client.verify_kyc(&user);
        assert!(client.is_kyc_verified(&user));

        // Issue series
        let par = 1_000_000i128;
        let sub_price = 980_000i128;
        let maturity = env.ledger().timestamp() + 90 * 24 * 3600;
        let max_cap = 10_000_000i128;
        let per_user_cap = 1_000_000i128;

        let series_id = client.issue_series(
            &par,
            &sub_price,
            &maturity,
            &max_cap,
            &per_user_cap,
            &usdc_token,
        );

        // Check series
        let series = client.get_series(&series_id);
        assert_eq!(series.par_value, par);
        assert_eq!(series.subscription_price, sub_price);

        // Check user has no position yet
        let position = client.get_user_position(&series_id, &user);
        assert_eq!(position.shares, 0);
    }

    #[test]
    fn test_kyc_enforcement() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BingoSeries);
        let client = BingoSeriesClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        client.initialize(&admin);

        // User not KYC verified
        assert!(!client.is_kyc_verified(&user));

        // Verify
        client.verify_kyc(&user);
        assert!(client.is_kyc_verified(&user));

        // Revoke
        client.revoke_kyc(&user);
        assert!(!client.is_kyc_verified(&user));
    }

    #[test]
    #[should_panic(expected = "Already initialized")]
    fn test_double_initialize() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BingoSeries);
        let client = BingoSeriesClient::new(&env, &contract_id);

        let admin = Address::generate(&env);

        client.initialize(&admin);
        client.initialize(&admin); // Should panic
    }

    #[test]
    #[should_panic(expected = "Subscription price must be less than par")]
    fn test_invalid_series_params() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BingoSeries);
        let client = BingoSeriesClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let usdc_token = Address::generate(&env);

        client.initialize(&admin);

        // Invalid: sub_price >= par
        client.issue_series(
            &1_000_000,
            &1_000_000, // Same as par
            &(env.ledger().timestamp() + 1000),
            &10_000_000,
            &1_000_000,
            &usdc_token,
        );
    }
}
