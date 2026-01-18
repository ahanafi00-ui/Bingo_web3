#![no_std]

mod error;
mod events;
mod storage;
mod validation;

use error::Error;
use events::*;
use storage::{DataKey, RepoPosition, RepoStatus};
use validation::{calculate_max_cash, calculate_repurchase};

use soroban_sdk::{contract, contractimpl, token, vec, Address, Env, IntoVal, Symbol};

#[contract]
pub struct RepoMarket;

#[contractimpl]
impl RepoMarket {
    // ============================================
    // INITIALIZATION & ADMIN
    // ============================================

    pub fn initialize(
        env: Env,
        admin: Address,
        treasury: Address,
        vault: Address,
        bt_bill_token: Address,
        stablecoin: Address,
        haircut_bps: i128,
        spread_bps: i128,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.storage().instance().set(&DataKey::Vault, &vault);
        env.storage().instance().set(&DataKey::BTBillToken, &bt_bill_token);
        env.storage().instance().set(&DataKey::Stablecoin, &stablecoin);
        env.storage().instance().set(&DataKey::Haircut, &haircut_bps);
        env.storage().instance().set(&DataKey::Spread, &spread_bps);
        env.storage().instance().set(&DataKey::PositionCounter, &0u64);
        env.storage().instance().set(&DataKey::Paused, &false);

        Ok(())
    }

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
    // FLOW 6: OPEN REPO
    // ============================================

    pub fn open_repo(
        env: Env,
        borrower: Address,
        series_id: u32,
        collateral_par: i128,
        desired_cash_out: i128,
        deadline: u64,
    ) -> Result<u64, Error> {
        Self::check_not_paused(&env)?;

        if collateral_par <= 0 || desired_cash_out <= 0 {
            return Err(Error::InvalidAmount);
        }

        borrower.require_auth();

        let vault: Address = env
            .storage()
            .instance()
            .get(&DataKey::Vault)
            .ok_or(Error::NotInitialized)?;

        let series: (u32, u64, u64, i128, i128, i128, i128, i128, u32) = env.invoke_contract(
            &vault,
            &Symbol::new(&env, "get_series"),
            vec![&env, series_id.into()],
        );
        let maturity_date = series.2;

        if deadline > maturity_date {
            return Err(Error::InvalidDeadline);
        }

        let mark_price: i128 = env.invoke_contract(
            &vault,
            &Symbol::new(&env, "current_price"),
            vec![&env, series_id.into()],
        );

        let haircut_bps: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Haircut)
            .unwrap_or(300);

        let max_cash =
            calculate_max_cash(collateral_par, mark_price, haircut_bps).ok_or(Error::InvalidAmount)?;

        if desired_cash_out > max_cash {
            return Err(Error::ExceedsMaxCash);
        }

        let spread_bps: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Spread)
            .unwrap_or(200);

        let repurchase_amount =
            calculate_repurchase(desired_cash_out, spread_bps).ok_or(Error::InvalidAmount)?;

        let bt_bill_token: Address = env
            .storage()
            .instance()
            .get(&DataKey::BTBillToken)
            .ok_or(Error::NotInitialized)?;

        env.invoke_contract::<()>(
            &bt_bill_token,
            &Symbol::new(&env, "transfer"),
            vec![
                &env,
                series_id.into(),
                borrower.to_val(),
                env.current_contract_address().to_val(),
                collateral_par.into_val(&env)
            ],
        );

        let stablecoin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Stablecoin)
            .ok_or(Error::NotInitialized)?;
        let treasury: Address = env
            .storage()
            .instance()
            .get(&DataKey::Treasury)
            .ok_or(Error::NotInitialized)?;

        let stablecoin_client = token::Client::new(&env, &stablecoin);
        stablecoin_client.transfer(&treasury, &borrower, &desired_cash_out);

        let position_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PositionCounter)
            .unwrap_or(0);
        let new_position_id = position_id + 1;

        let position = RepoPosition {
            id: new_position_id,
            borrower: borrower.clone(),
            series_id,
            collateral_par,
            cash_out: desired_cash_out,
            repurchase_amount,
            start_time: env.ledger().timestamp(),
            deadline,
            status: RepoStatus::Open,
        };

        env.storage()
            .instance()
            .set(&DataKey::Position(new_position_id), &position);
        env.storage()
            .instance()
            .set(&DataKey::PositionCounter, &new_position_id);

        env.events().publish(
            (Symbol::new(&env, "repo_opened"), new_position_id),
            RepoOpenedEvent {
                position_id: new_position_id,
                borrower: borrower.clone(),
                series_id,
                collateral_par,
                cash_out: desired_cash_out,
                repurchase_amount,
                deadline,
            },
        );

        Ok(new_position_id)
    }

    // ============================================
    // FLOW 7: CLOSE REPO (REPAY)
    // ============================================

    pub fn close_repo(env: Env, position_id: u64) -> Result<(), Error> {
        Self::check_not_paused(&env)?;

        let mut position: RepoPosition = env
            .storage()
            .instance()
            .get(&DataKey::Position(position_id))
            .ok_or(Error::PositionNotFound)?;

        if position.status != RepoStatus::Open {
            return Err(Error::InvalidStatus);
        }

        position.borrower.require_auth();

        let current_time = env.ledger().timestamp();
        if current_time > position.deadline {
            return Err(Error::DeadlinePassed);
        }

        let stablecoin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Stablecoin)
            .ok_or(Error::NotInitialized)?;
        let treasury: Address = env
            .storage()
            .instance()
            .get(&DataKey::Treasury)
            .ok_or(Error::NotInitialized)?;

        let stablecoin_client = token::Client::new(&env, &stablecoin);
        stablecoin_client.transfer(&position.borrower, &treasury, &position.repurchase_amount);

        let bt_bill_token: Address = env
            .storage()
            .instance()
            .get(&DataKey::BTBillToken)
            .ok_or(Error::NotInitialized)?;

        env.invoke_contract::<()>(
            &bt_bill_token,
            &Symbol::new(&env, "transfer"),
            vec![
                &env,
                position.series_id.into(),
                env.current_contract_address().to_val(),
                position.borrower.to_val(),
                position.collateral_par.into_val(&env)
            ],
        );

        position.status = RepoStatus::Closed;
        env.storage()
            .instance()
            .set(&DataKey::Position(position_id), &position);

        env.events().publish(
            (Symbol::new(&env, "repo_closed"), position_id),
            RepoClosedEvent {
                position_id,
                borrower: position.borrower.clone(),
                repayment: position.repurchase_amount,
            },
        );

        Ok(())
    }

    // ============================================
    // FLOW 8: CLAIM DEFAULT
    // ============================================

    pub fn claim_default(env: Env, position_id: u64) -> Result<(), Error> {
        Self::check_not_paused(&env)?;

        let treasury: Address = env
            .storage()
            .instance()
            .get(&DataKey::Treasury)
            .ok_or(Error::NotInitialized)?;

        treasury.require_auth();

        let mut position: RepoPosition = env
            .storage()
            .instance()
            .get(&DataKey::Position(position_id))
            .ok_or(Error::PositionNotFound)?;

        if position.status != RepoStatus::Open {
            return Err(Error::InvalidStatus);
        }

        let current_time = env.ledger().timestamp();
        if current_time <= position.deadline {
            return Err(Error::DeadlineNotPassed);
        }

        let bt_bill_token: Address = env
            .storage()
            .instance()
            .get(&DataKey::BTBillToken)
            .ok_or(Error::NotInitialized)?;

        env.invoke_contract::<()>(
            &bt_bill_token,
            &Symbol::new(&env, "transfer"),
            vec![
                &env,
                position.series_id.into(),
                env.current_contract_address().to_val(),
                treasury.to_val(),
                position.collateral_par.into_val(&env)
            ],
        );

        position.status = RepoStatus::Defaulted;
        env.storage()
            .instance()
            .set(&DataKey::Position(position_id), &position);

        env.events().publish(
            (Symbol::new(&env, "repo_defaulted"), position_id),
            RepoDefaultedEvent {
                position_id,
                borrower: position.borrower.clone(),
                treasury: treasury.clone(),
                collateral_claimed: position.collateral_par,
            },
        );

        Ok(())
    }

    // ============================================
    // VIEW FUNCTIONS
    // ============================================

    pub fn get_position(env: Env, position_id: u64) -> Result<RepoPosition, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Position(position_id))
            .ok_or(Error::PositionNotFound)
    }

    pub fn get_haircut(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Haircut)
            .unwrap_or(300)
    }

    pub fn get_spread(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Spread)
            .unwrap_or(200)
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
}
