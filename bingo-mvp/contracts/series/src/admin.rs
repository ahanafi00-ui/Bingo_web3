use crate::storage::Storage;
use crate::types::{Series, SeriesStatus, SCALE};
use crate::validation::Validator;
use soroban_sdk::{Address, Env};

pub struct Admin;

impl Admin {
    /// Initialize contract with admin
    pub fn initialize(env: &Env, admin: &Address) {
        if Storage::has_admin(env) {
            panic!("Already initialized");
        }
        Storage::set_admin(env, admin);
    }

    /// Issue a new obligation series (Admin only)
    pub fn issue_series(
        env: &Env,
        par_value: i128,
        subscription_price: i128,
        maturity_time: u64,
        max_cap: i128,
        per_user_cap: i128,
        usdc_token: &Address,
    ) -> u32 {
        // Auth check
        let admin = Storage::get_admin(env);
        admin.require_auth();

        // Validate inputs
        Validator::validate_series_params(
            env,
            par_value,
            subscription_price,
            maturity_time,
            max_cap,
            per_user_cap,
        );

        // Get next series ID
        let series_id = Storage::get_next_series_id(env);

        // Create series
        let series = Series {
            id: series_id,
            par_value,
            subscription_price,
            issue_time: env.ledger().timestamp(),
            maturity_time,
            max_cap,
            per_user_cap,
            total_subscribed: 0,
            status: SeriesStatus::Active,
            usdc_token: usdc_token.clone(),
        };

        // Store series
        Storage::set_series(env, &series);

        // Increment next ID
        Storage::increment_series_id(env);

        series_id
    }

    /// Verify user KYC
    pub fn verify_kyc(env: &Env, user: &Address) {
        let admin = Storage::get_admin(env);
        admin.require_auth();

        Storage::set_kyc_verified(env, user, true);
    }

    /// Revoke user KYC
    pub fn revoke_kyc(env: &Env, user: &Address) {
        let admin = Storage::get_admin(env);
        admin.require_auth();

        Storage::set_kyc_verified(env, user, false);
    }

    /// Settle matured series (deposit USDC for redemptions)
    pub fn settle_series(env: &Env, series_id: u32, usdc_amount: i128, admin: &Address) {
        admin.require_auth();

        let admin_stored = Storage::get_admin(env);
        if admin != &admin_stored {
            panic!("Not admin");
        }

        let mut series = Storage::get_series(env, series_id);

        // Calculate required amount
        let required = (series.total_subscribed * series.par_value) / SCALE;

        // Validate
        Validator::validate_settlement(env, &series, usdc_amount, required);

        // Transfer USDC from admin to contract
        let usdc_client = soroban_sdk::token::Client::new(env, &series.usdc_token);
        usdc_client.transfer(admin, &env.current_contract_address(), &usdc_amount);

        // Update status
        series.status = SeriesStatus::Settled;
        Storage::set_series(env, &series);
    }
}
