#![no_std]

mod error;
mod events;
mod pricing;
mod storage;

use error::Error;
use events::*;
use pricing::{calculate_current_price, calculate_minted_par};
use storage::{DataKey, PAR_UNIT, Series, SeriesStatus, UserPosition};

use soroban_sdk::{contract, contractimpl, token, vec, Address, Env, IntoVal, Symbol};

#[contract]
pub struct BingoVault;

#[contractimpl]
impl BingoVault {
    // ============================================
    // INITIALIZATION & ADMIN
    // ============================================

    /// Initialize the vault
    ///
    /// # Errors
    /// - `AlreadyInitialized`: Contract already initialized
    pub fn initialize(
        env: Env,
        admin: Address,
        treasury: Address,
        stablecoin: Address,
        bt_bill_token: Address,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.storage().instance().set(&DataKey::Stablecoin, &stablecoin);
        env.storage()
            .instance()
            .set(&DataKey::BTBillToken, &bt_bill_token);
        env.storage().instance().set(&DataKey::Paused, &false);

        Ok(())
    }

    /// Pause contract (emergency)
    ///
    /// # Errors
    /// - `NotInitialized`: Contract not initialized
    /// - `Unauthorized`: Caller is not admin
    pub fn pause(env: Env) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        env.storage().instance().set(&DataKey::Paused, &true);
        Ok(())
    }

    /// Unpause contract
    ///
    /// # Errors
    /// - `NotInitialized`: Contract not initialized
    /// - `Unauthorized`: Caller is not admin
    pub fn unpause(env: Env) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        env.storage().instance().set(&DataKey::Paused, &false);
        Ok(())
    }

    // ============================================
    // FLOW 1: TREASURY CREATES SERIES
    // ============================================

    /// Create a new T-Bill series
    ///
    /// # Errors
    /// - `NotInitialized`: Contract not initialized
    /// - `Unauthorized`: Caller is not treasury
    /// - `ContractPaused`: Contract is paused
    /// - `SeriesAlreadyExists`: Series ID already used
    /// - `InvalidTimestamp`: Maturity must be after issue date
    /// - `InvalidIssuePrice`: Price must be in range (0, PAR]
    /// - `InvalidCapAmounts`: user_cap must be ≤ series_cap, both positive
    pub fn create_series(
        env: Env,
        series_id: u32,
        issue_date: u64,
        maturity_date: u64,
        issue_price: i128,
        cap_par: i128,
        user_cap_par: i128,
    ) -> Result<(), Error> {
        Self::check_not_paused(&env)?;

        let treasury: Address = env
            .storage()
            .instance()
            .get(&DataKey::Treasury)
            .ok_or(Error::NotInitialized)?;

        // Treasury must authorize this
        treasury.require_auth();

        // Validate: Series doesn't already exist
        if env
            .storage()
            .instance()
            .has(&DataKey::Series(series_id))
        {
            return Err(Error::SeriesAlreadyExists);
        }

        // Validate: Maturity after issue
        if maturity_date <= issue_date {
            return Err(Error::InvalidTimestamp);
        }

        // Validate: Issue price in valid range (0, PAR]
        if issue_price <= 0 || issue_price > PAR_UNIT {
            return Err(Error::InvalidIssuePrice);
        }

        // Validate: Cap amounts are valid
        if cap_par <= 0 || user_cap_par <= 0 || user_cap_par > cap_par {
            return Err(Error::InvalidCapAmounts);
        }

        let series = Series {
            series_id,
            issue_date,
            maturity_date,
            par_unit: PAR_UNIT,
            issue_price,
            cap_par,
            minted_par: 0,
            user_cap_par,
            status: SeriesStatus::Upcoming,
            total_subscriptions_collected: 0,
        };

        env.storage()
            .instance()
            .set(&DataKey::Series(series_id), &series);

        env.events().publish(
            (Symbol::new(&env, "series_created"), series_id),
            SeriesCreatedEvent {
                series_id,
                issue_date,
                maturity_date,
                issue_price,
                cap_par,
                user_cap_par,
            },
        );

        Ok(())
    }

    // ============================================
    // FLOW 2: TREASURY ACTIVATES SERIES
    // ============================================

    /// Activate a series for subscriptions
    ///
    /// # Errors
    /// - `NotInitialized`: Contract not initialized
    /// - `Unauthorized`: Caller is not treasury
    /// - `ContractPaused`: Contract is paused
    /// - `SeriesNotFound`: Series doesn't exist
    /// - `InvalidStatus`: Series not in UPCOMING status
    pub fn activate_series(env: Env, series_id: u32) -> Result<(), Error> {
        Self::check_not_paused(&env)?;

        let treasury: Address = env
            .storage()
            .instance()
            .get(&DataKey::Treasury)
            .ok_or(Error::NotInitialized)?;

        // Treasury must authorize this
        treasury.require_auth();

        let mut series: Series = env
            .storage()
            .instance()
            .get(&DataKey::Series(series_id))
            .ok_or(Error::SeriesNotFound)?;

        // Validate: Must be UPCOMING status
        if series.status != SeriesStatus::Upcoming {
            return Err(Error::InvalidStatus);
        }

        series.status = SeriesStatus::Active;
        env.storage()
            .instance()
            .set(&DataKey::Series(series_id), &series);

        env.events().publish(
            (Symbol::new(&env, "series_activated"), series_id),
            SeriesActivatedEvent { series_id },
        );

        Ok(())
    }

    // ============================================
    // FLOW 4: USER SUBSCRIBES (BUYS T-BILLS)
    // ============================================

    /// Subscribe to a series (buy bT-Bills)
    ///
    /// # Errors
    /// - `NotInitialized`: Contract not initialized
    /// - `ContractPaused`: Contract is paused
    /// - `InvalidAmount`: pay_amount must be positive
    /// - `SeriesNotFound`: Series doesn't exist
    /// - `SeriesNotActive`: Series not yet activated or already matured
    /// - `ExceedsSeriesCap`: Would exceed series cap_par limit
    /// - `ExceedsUserCap`: Would exceed user's personal cap_par limit
    pub fn subscribe(
        env: Env,
        user: Address,
        series_id: u32,
        pay_amount: i128,
    ) -> Result<(), Error> {
        Self::check_not_paused(&env)?;

        if pay_amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        user.require_auth();

        let mut series: Series = env
            .storage()
            .instance()
            .get(&DataKey::Series(series_id))
            .ok_or(Error::SeriesNotFound)?;

        // Validate: Series must be ACTIVE
        if series.status != SeriesStatus::Active {
            return Err(Error::SeriesNotActive);
        }

        // Calculate current price (linear accretion)
        let current_time = env.ledger().timestamp();
        let current_price = calculate_current_price(&series, current_time);

        // Calculate how many PAR units to mint
        let minted_par =
            calculate_minted_par(pay_amount, current_price).ok_or(Error::InvalidAmount)?;

        // Validate: Series cap
        let new_series_minted = series
            .minted_par
            .checked_add(minted_par)
            .ok_or(Error::InvalidAmount)?;

        if new_series_minted > series.cap_par {
            return Err(Error::ExceedsSeriesCap);
        }

        // Validate: User cap
        let user_position_key = DataKey::UserPosition(series_id, user.clone());
        let mut user_position = env
            .storage()
            .instance()
            .get::<DataKey, UserPosition>(&user_position_key)
            .unwrap_or(UserPosition { subscribed_par: 0 });

        let new_user_subscribed = user_position
            .subscribed_par
            .checked_add(minted_par)
            .ok_or(Error::InvalidAmount)?;

        if new_user_subscribed > series.user_cap_par {
            return Err(Error::ExceedsUserCap);
        }

        // Transfer stablecoin from user to vault
        let stablecoin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Stablecoin)
            .ok_or(Error::NotInitialized)?;

        let stablecoin_client = token::Client::new(&env, &stablecoin);
        stablecoin_client.transfer(&user, &env.current_contract_address(), &pay_amount);

        // Mint bT-Bills
        let bt_bill_token: Address = env
            .storage()
            .instance()
            .get(&DataKey::BTBillToken)
            .ok_or(Error::NotInitialized)?;

        env.invoke_contract::<()>(
            &bt_bill_token,
            &Symbol::new(&env, "mint"),
            vec![
                &env,
                series_id.into(),
                user.to_val(),
                minted_par.into_val(&env)
            ],
        );

        // Update state
        series.minted_par = new_series_minted;
        series.total_subscriptions_collected = series
            .total_subscriptions_collected
            .checked_add(pay_amount)
            .ok_or(Error::InvalidAmount)?;
        
        user_position.subscribed_par = new_user_subscribed;

        env.storage()
            .instance()
            .set(&DataKey::Series(series_id), &series);
        env.storage()
            .instance()
            .set(&user_position_key, &user_position);

        // Update protocol accounting
        use storage::ProtocolAccounting;
        let mut accounting = env
            .storage()
            .instance()
            .get::<DataKey, ProtocolAccounting>(&DataKey::ProtocolAccounting)
            .unwrap_or(ProtocolAccounting {
                total_subscriptions_collected: 0,
                total_par_minted: 0,
                total_lent: 0,
                total_repo_revenue: 0,
                total_defaults: 0,
            });

        accounting.total_subscriptions_collected = accounting
            .total_subscriptions_collected
            .checked_add(pay_amount)
            .ok_or(Error::InvalidAmount)?;
        accounting.total_par_minted = accounting
            .total_par_minted
            .checked_add(minted_par)
            .ok_or(Error::InvalidAmount)?;

        env.storage()
            .instance()
            .set(&DataKey::ProtocolAccounting, &accounting);

        env.events().publish(
            (Symbol::new(&env, "subscribed"), series_id, user.clone()),
            SubscribedEvent {
                series_id,
                user,
                pay_amount,
                minted_par,
                price: current_price,
            },
        );

        Ok(())
    }

    // ============================================
    // FLOW 8: USER REDEEMS AT MATURITY
    // ============================================

    /// Redeem bT-Bills at maturity for PAR value
    ///
    /// # Errors
    /// - `NotInitialized`: Contract not initialized
    /// - `ContractPaused`: Contract is paused
    /// - `InvalidAmount`: bt_bill_amount must be positive
    /// - `SeriesNotFound`: Series doesn't exist
    /// - `SeriesNotMatured`: Cannot redeem before maturity_date
    /// - `InsufficientBalance`: User doesn't have enough bT-Bills
    pub fn redeem(
        env: Env,
        user: Address,
        series_id: u32,
        bt_bill_amount: i128,
    ) -> Result<(), Error> {
        Self::check_not_paused(&env)?;

        if bt_bill_amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        user.require_auth();

        let series: Series = env
            .storage()
            .instance()
            .get(&DataKey::Series(series_id))
            .ok_or(Error::SeriesNotFound)?;

        // Validate: Must be at or past maturity
        let current_time = env.ledger().timestamp();
        if current_time < series.maturity_date {
            return Err(Error::SeriesNotMatured);
        }

        // Burn bT-Bills
        let bt_bill_token: Address = env
            .storage()
            .instance()
            .get(&DataKey::BTBillToken)
            .ok_or(Error::NotInitialized)?;

        env.invoke_contract::<()>(
            &bt_bill_token,
            &Symbol::new(&env, "burn"),
            vec![
                &env,
                series_id.into(),
                user.to_val(),
                bt_bill_amount.into_val(&env)
            ],
        );

        // Transfer stablecoin from vault to user (1:1 PAR value)
        let stablecoin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Stablecoin)
            .ok_or(Error::NotInitialized)?;

        let stablecoin_client = token::Client::new(&env, &stablecoin);
        stablecoin_client.transfer(&env.current_contract_address(), &user, &bt_bill_amount);

        env.events().publish(
            (Symbol::new(&env, "redeemed"), series_id, user.clone()),
            RedeemedEvent {
                series_id,
                user,
                bt_bill_amount,
                payout: bt_bill_amount,
            },
        );

        Ok(())
    }

    // ============================================
    // VIEW FUNCTIONS
    // ============================================

    /// Get current price for a series
    pub fn current_price(env: Env, series_id: u32) -> Result<i128, Error> {
        let series: Series = env
            .storage()
            .instance()
            .get(&DataKey::Series(series_id))
            .ok_or(Error::SeriesNotFound)?;

        let current_time = env.ledger().timestamp();
        Ok(calculate_current_price(&series, current_time))
    }

    /// Get series details
    pub fn get_series(env: Env, series_id: u32) -> Result<Series, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Series(series_id))
            .ok_or(Error::SeriesNotFound)
    }

    /// Get user position in a series
    pub fn get_user_position(env: Env, series_id: u32, user: Address) -> UserPosition {
        env.storage()
            .instance()
            .get::<DataKey, UserPosition>(&DataKey::UserPosition(series_id, user))
            .unwrap_or(UserPosition { subscribed_par: 0 })
    }

    /// Get protocol accounting (revenue tracking)
    pub fn get_protocol_accounting(env: Env) -> storage::ProtocolAccounting {
        use storage::ProtocolAccounting;
        env.storage()
            .instance()
            .get::<DataKey, ProtocolAccounting>(&DataKey::ProtocolAccounting)
            .unwrap_or(ProtocolAccounting {
                total_subscriptions_collected: 0,
                total_par_minted: 0,
                total_lent: 0,
                total_repo_revenue: 0,
                total_defaults: 0,
            })
    }

    /// Calculate protocol profit (subscriptions + repo - redemption liability)
    /// Note: This is unrealized profit until maturity
    pub fn calculate_protocol_profit(env: Env) -> i128 {
        use storage::ProtocolAccounting;
        let accounting = env
            .storage()
            .instance()
            .get::<DataKey, ProtocolAccounting>(&DataKey::ProtocolAccounting)
            .unwrap_or(ProtocolAccounting {
                total_subscriptions_collected: 0,
                total_par_minted: 0,
                total_lent: 0,
                total_repo_revenue: 0,
                total_defaults: 0,
            });

        // Revenue = subscriptions + repo profits
        let revenue = accounting
            .total_subscriptions_collected
            .checked_add(accounting.total_repo_revenue)
            .unwrap_or(0);

        // Liability = PAR minted (will need to pay at maturity)
        let liability = accounting.total_par_minted;

        // Profit (can be negative early on)
        revenue.saturating_sub(liability)
    }

    /// Calculate available USDC for repo lending
    /// 
    /// With 100% liquidity model: ALL vault USDC is available
    /// Safety ensured by haircut on each repo position
    pub fn calculate_available_for_lending(env: Env) -> i128 {
        use storage::ProtocolAccounting;
        let accounting = env
            .storage()
            .instance()
            .get::<DataKey, ProtocolAccounting>(&DataKey::ProtocolAccounting)
            .unwrap_or(ProtocolAccounting {
                total_subscriptions_collected: 0,
                total_par_minted: 0,
                total_lent: 0,
                total_repo_revenue: 0,
                total_defaults: 0,
            });

        // Total USDC in vault = subscriptions + repo returns
        let total_usdc = accounting
            .total_subscriptions_collected
            .checked_add(accounting.total_repo_revenue)
            .unwrap_or(0);

        // Currently lent out
        let lent = accounting.total_lent;

        // Available = total - lent (100% of remaining balance)
        total_usdc.saturating_sub(lent)
    }

    // ============================================
    // INTERNAL HELPERS
    // ============================================

    fn check_not_paused(env: &Env) -> Result<(), Error> {
        let paused = env
            .storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::Paused)
            .unwrap_or(false);

        if paused {
            return Err(Error::ContractPaused);
        }
        Ok(())
    }

    /// Mark series as matured (can be called by anyone at maturity)
    pub fn mature_series(env: Env, series_id: u32) -> Result<(), Error> {
        let mut series: Series = env
            .storage()
            .instance()
            .get(&DataKey::Series(series_id))
            .ok_or(Error::SeriesNotFound)?;

        let current_time = env.ledger().timestamp();
        if current_time < series.maturity_date {
            return Err(Error::SeriesNotMatured);
        }

        if series.status != SeriesStatus::Active {
            return Err(Error::InvalidStatus);
        }

        series.status = SeriesStatus::Matured;
        env.storage()
            .instance()
            .set(&DataKey::Series(series_id), &series);

        env.events().publish(
            (Symbol::new(&env, "series_matured"), series_id),
            SeriesMaturedEvent { series_id },
        );

        Ok(())
    }
}
