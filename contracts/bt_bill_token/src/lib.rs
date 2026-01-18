#![no_std]

mod error;
mod events;
mod storage;

use error::Error;
use events::{BurnEvent, MintEvent, TransferEvent};
use storage::{Balance, DataKey};

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

#[contract]
pub struct BTBillToken;

#[contractimpl]
impl BTBillToken {
    /// Initialize the token contract
    /// 
    /// # Errors
    /// - `AlreadyInitialized`: Contract already initialized
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Admin, &admin);

        Ok(())
    }

    /// Add an operator (vault or repo_market contract)
    /// 
    /// # Errors
    /// - `NotInitialized`: Contract not initialized
    /// - `Unauthorized`: Caller is not admin
    pub fn add_operator(env: Env, operator: Address) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::Operators(operator.clone()), &true);

        Ok(())
    }

    /// Remove an operator
    /// 
    /// # Errors
    /// - `NotInitialized`: Contract not initialized
    /// - `Unauthorized`: Caller is not admin
    pub fn remove_operator(env: Env, operator: Address) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        env.storage()
            .instance()
            .remove(&DataKey::Operators(operator));

        Ok(())
    }

    /// Mint tokens (only operators)
    /// 
    /// # Errors
    /// - `NotInitialized`: Contract not initialized
    /// - `InvalidAmount`: Amount <= 0
    pub fn mint(env: Env, series_id: u32, to: Address, amount: i128) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::NotInitialized);
        }

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let balance_key = DataKey::Balance(series_id, to.clone());
        let current_balance = env
            .storage()
            .instance()
            .get::<DataKey, Balance>(&balance_key)
            .unwrap_or(Balance { amount: 0 });

        let new_balance = current_balance
            .amount
            .checked_add(amount)
            .ok_or(Error::InvalidAmount)?;

        env.storage().instance().set(
            &balance_key,
            &Balance {
                amount: new_balance,
            },
        );

        env.events().publish(
            (Symbol::new(&env, "mint"), series_id),
            MintEvent {
                series_id,
                to: to.clone(),
                amount,
            },
        );

        Ok(())
    }

    /// Burn tokens (only operators)
    /// 
    /// # Errors
    /// - `NotInitialized`: Contract not initialized
    /// - `InvalidAmount`: Amount <= 0
    /// - `InsufficientBalance`: Not enough balance
    pub fn burn(env: Env, series_id: u32, from: Address, amount: i128) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::NotInitialized);
        }

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let balance_key = DataKey::Balance(series_id, from.clone());
        let current_balance = env
            .storage()
            .instance()
            .get::<DataKey, Balance>(&balance_key)
            .ok_or(Error::InsufficientBalance)?;

        if current_balance.amount < amount {
            return Err(Error::InsufficientBalance);
        }

        let new_balance = current_balance.amount - amount;

        if new_balance == 0 {
            env.storage().instance().remove(&balance_key);
        } else {
            env.storage().instance().set(
                &balance_key,
                &Balance {
                    amount: new_balance,
                },
            );
        }

        env.events().publish(
            (Symbol::new(&env, "burn"), series_id),
            BurnEvent {
                series_id,
                from: from.clone(),
                amount,
            },
        );

        Ok(())
    }

    /// Transfer tokens between users
    /// 
    /// # Errors
    /// - `NotInitialized`: Contract not initialized
    /// - `InvalidAmount`: Amount <= 0
    /// - `Unauthorized`: From address did not authorize
    /// - `InsufficientBalance`: Not enough balance
    pub fn transfer(
        env: Env,
        series_id: u32,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::NotInitialized);
        }

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        from.require_auth();

        let from_key = DataKey::Balance(series_id, from.clone());
        let from_balance = env
            .storage()
            .instance()
            .get::<DataKey, Balance>(&from_key)
            .ok_or(Error::InsufficientBalance)?;

        if from_balance.amount < amount {
            return Err(Error::InsufficientBalance);
        }

        let to_key = DataKey::Balance(series_id, to.clone());
        let to_balance = env
            .storage()
            .instance()
            .get::<DataKey, Balance>(&to_key)
            .unwrap_or(Balance { amount: 0 });

        let new_from_balance = from_balance.amount - amount;
        let new_to_balance = to_balance
            .amount
            .checked_add(amount)
            .ok_or(Error::InvalidAmount)?;

        if new_from_balance == 0 {
            env.storage().instance().remove(&from_key);
        } else {
            env.storage().instance().set(
                &from_key,
                &Balance {
                    amount: new_from_balance,
                },
            );
        }

        env.storage().instance().set(
            &to_key,
            &Balance {
                amount: new_to_balance,
            },
        );

        env.events().publish(
            (Symbol::new(&env, "transfer"), series_id),
            TransferEvent {
                series_id,
                from: from.clone(),
                to: to.clone(),
                amount,
            },
        );

        Ok(())
    }

    /// Get balance for a user in a series
    pub fn balance_of(env: Env, series_id: u32, user: Address) -> i128 {
        env.storage()
            .instance()
            .get::<DataKey, Balance>(&DataKey::Balance(series_id, user))
            .unwrap_or(Balance { amount: 0 })
            .amount
    }

    /// Check if address is an operator
    pub fn is_operator(env: Env, address: Address) -> bool {
        env.storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::Operators(address))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    const SCALE: i128 = 10_000_000;

    #[test]
    fn test_initialize() {
        let env = Env::default();
        let contract_id = env.register_contract(None, BTBillToken);
        let client = BTBillTokenClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin);

        let result = client.try_initialize(&admin);
        assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
    }

    #[test]
    fn test_mint_and_balance() {
        let env = Env::default();
        let contract_id = env.register_contract(None, BTBillToken);
        let client = BTBillTokenClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        client.initialize(&admin);

        let series_id = 1u32;
        let amount = 1000i128 * SCALE;

        client.mint(&series_id, &user, &amount);

        let balance = client.balance_of(&series_id, &user);
        assert_eq!(balance, amount);
    }

    #[test]
    fn test_transfer() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BTBillToken);
        let client = BTBillTokenClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);

        client.initialize(&admin);

        let series_id = 1u32;
        let amount = 1000i128 * SCALE;

        client.mint(&series_id, &user1, &amount);
        client.transfer(&series_id, &user1, &user2, &(500i128 * SCALE));

        assert_eq!(client.balance_of(&series_id, &user1), 500i128 * SCALE);
        assert_eq!(client.balance_of(&series_id, &user2), 500i128 * SCALE);
    }

    #[test]
    fn test_burn() {
        let env = Env::default();
        let contract_id = env.register_contract(None, BTBillToken);
        let client = BTBillTokenClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        client.initialize(&admin);

        let series_id = 1u32;
        let amount = 1000i128 * SCALE;

        client.mint(&series_id, &user, &amount);
        client.burn(&series_id, &user, &(400i128 * SCALE));

        assert_eq!(client.balance_of(&series_id, &user), 600i128 * SCALE);
    }

    #[test]
    fn test_insufficient_balance_error() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, BTBillToken);
        let client = BTBillTokenClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);

        client.initialize(&admin);

        let series_id = 1u32;
        let amount = 1000i128 * SCALE;

        client.mint(&series_id, &user1, &amount);

        let result = client.try_transfer(&series_id, &user1, &user2, &(1500i128 * SCALE));
        assert_eq!(result, Err(Ok(Error::InsufficientBalance)));
    }
}
