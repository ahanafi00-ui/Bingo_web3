use crate::types::{DataKey, Series, UserPosition};
use soroban_sdk::{Address, Env};

pub struct Storage;

impl Storage {
    // Admin
    pub fn get_admin(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin not set")
    }

    pub fn set_admin(env: &Env, admin: &Address) {
        env.storage().instance().set(&DataKey::Admin, admin);
    }

    pub fn has_admin(env: &Env) -> bool {
        env.storage().instance().has(&DataKey::Admin)
    }

    // Series ID counter
    pub fn get_next_series_id(env: &Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::NextSeriesId)
            .unwrap_or(0)
    }

    pub fn increment_series_id(env: &Env) {
        let current = Self::get_next_series_id(env);
        env.storage()
            .instance()
            .set(&DataKey::NextSeriesId, &(current + 1));
    }

    // Series
    pub fn get_series(env: &Env, series_id: u32) -> Series {
        env.storage()
            .persistent()
            .get(&DataKey::Series(series_id))
            .expect("Series not found")
    }

    pub fn set_series(env: &Env, series: &Series) {
        env.storage()
            .persistent()
            .set(&DataKey::Series(series.id), series);
    }

    pub fn has_series(env: &Env, series_id: u32) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Series(series_id))
    }

    // User Position
    pub fn get_user_position(env: &Env, series_id: u32, user: &Address) -> Option<UserPosition> {
        env.storage()
            .persistent()
            .get(&DataKey::UserPosition(series_id, user.clone()))
    }

    pub fn set_user_position(env: &Env, series_id: u32, user: &Address, position: &UserPosition) {
        env.storage()
            .persistent()
            .set(&DataKey::UserPosition(series_id, user.clone()), position);
    }

    pub fn remove_user_position(env: &Env, series_id: u32, user: &Address) {
        env.storage()
            .persistent()
            .remove(&DataKey::UserPosition(series_id, user.clone()));
    }

    // KYC
    pub fn is_kyc_verified(env: &Env, user: &Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::KYCVerified(user.clone()))
            .unwrap_or(false)
    }

    pub fn set_kyc_verified(env: &Env, user: &Address, verified: bool) {
        env.storage()
            .persistent()
            .set(&DataKey::KYCVerified(user.clone()), &verified);
    }
}
