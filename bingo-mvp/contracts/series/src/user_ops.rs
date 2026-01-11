use crate::storage::Storage;
use crate::types::{UserPosition, SCALE};
use crate::validation::Validator;
use crate::yield_calc::YieldCalculator;
use soroban_sdk::{token, Address, Env};

pub struct UserOps;

impl UserOps {
    /// Subscribe to a series (KYC verified users only)
    pub fn subscribe(env: &Env, series_id: u32, usdc_amount: i128, user: &Address) -> i128 {
        user.require_auth();

        // Check KYC
        if !Storage::is_kyc_verified(env, user) {
            panic!("User not KYC verified");
        }

        // Load series
        let mut series = Storage::get_series(env, series_id);

        // Calculate current index
        let current_index = YieldCalculator::calculate_index(env, &series);

        // Calculate shares to mint
        let shares = YieldCalculator::calculate_shares(usdc_amount, current_index);

        // Get existing position
        let existing_position = Storage::get_user_position(env, series_id, user)
            .unwrap_or(UserPosition {
                shares: 0,
                entry_index: current_index,
            });

        let new_total_shares = existing_position.shares + shares;

        // Validate subscription
        Validator::validate_subscription(env, &series, shares, new_total_shares);

        // Transfer USDC from user to contract
        let usdc_client = token::Client::new(env, &series.usdc_token);
        usdc_client.transfer(user, &env.current_contract_address(), &usdc_amount);

        // Update series total
        series.total_subscribed += shares;
        Storage::set_series(env, &series);

        // Update user position
        let user_position = UserPosition {
            shares: new_total_shares,
            entry_index: if existing_position.shares == 0 {
                current_index
            } else {
                existing_position.entry_index
            },
        };
        Storage::set_user_position(env, series_id, user, &user_position);

        shares
    }

    /// Redeem at maturity (KYC verified users only)
    pub fn redeem(env: &Env, series_id: u32, user: &Address) -> i128 {
        user.require_auth();

        // Check KYC
        if !Storage::is_kyc_verified(env, user) {
            panic!("User not KYC verified");
        }

        // Load series
        let series = Storage::get_series(env, series_id);

        // Validate redemption
        Validator::validate_redemption(env, &series);

        // Load user position
        let user_position = Storage::get_user_position(env, series_id, user)
            .expect("No position found");

        // Calculate redemption value (always par at maturity)
        let redemption_value = YieldCalculator::calculate_redemption_value(
            user_position.shares,
            series.par_value,
        );

        // Transfer USDC to user
        let usdc_client = token::Client::new(env, &series.usdc_token);
        usdc_client.transfer(&env.current_contract_address(), user, &redemption_value);

        // Clear user position
        Storage::remove_user_position(env, series_id, user);

        redemption_value
    }

    /// Get current value of user's position
    pub fn get_position_value(env: &Env, series_id: u32, user: &Address) -> i128 {
        let series = Storage::get_series(env, series_id);

        let user_position = Storage::get_user_position(env, series_id, user)
            .unwrap_or(UserPosition {
                shares: 0,
                entry_index: SCALE,
            });

        if user_position.shares == 0 {
            return 0;
        }

        let current_index = YieldCalculator::calculate_index(env, &series);
        YieldCalculator::calculate_position_value(user_position.shares, current_index)
    }
}
