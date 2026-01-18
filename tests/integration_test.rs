#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env, Symbol,
};

// Re-export contract clients
mod bt_bill_token {
    soroban_sdk::contractimport!(
        file = "../target/wasm32-unknown-unknown/release/bt_bill_token.wasm"
    );
}

mod bingo_vault {
    soroban_sdk::contractimport!(
        file = "../target/wasm32-unknown-unknown/release/bingo_vault.wasm"
    );
}

mod repo_market {
    soroban_sdk::contractimport!(
        file = "../target/wasm32-unknown-unknown/release/repo_market.wasm"
    );
}

// Constants
const SCALE: i128 = 10_000_000;
const PAR_UNIT: i128 = 1 * SCALE;

struct TestContext {
    env: Env,
    admin: Address,
    treasury: Address,
    user1: Address,
    user2: Address,
    stablecoin: Address,
    stablecoin_admin: Address,
    bt_bill_token_id: Address,
    vault_id: Address,
    repo_id: Address,
}

fn setup_test() -> TestContext {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let stablecoin_admin = Address::generate(&env);

    // Deploy stablecoin (use Stellar Asset Contract)
    let stablecoin_contract = env.register_stellar_asset_contract_v2(stablecoin_admin.clone());
    let stablecoin = stablecoin_contract.address();

    // Mint stablecoin to users and treasury
    let stablecoin_client = token::Client::new(&env, &stablecoin);
    stablecoin_client.mint(&user1, &(1_000_000i128 * SCALE));
    stablecoin_client.mint(&user2, &(1_000_000i128 * SCALE));
    stablecoin_client.mint(&treasury, &(10_000_000i128 * SCALE));

    // Deploy bt_bill_token
    let bt_bill_token_id = env.register_contract_wasm(None, bt_bill_token::WASM);
    let bt_bill_client = bt_bill_token::Client::new(&env, &bt_bill_token_id);
    bt_bill_client.initialize(&admin);

    // Deploy vault
    let vault_id = env.register_contract_wasm(None, bingo_vault::WASM);
    let vault_client = bingo_vault::Client::new(&env, &vault_id);
    vault_client.initialize(&admin, &treasury, &stablecoin, &bt_bill_token_id);

    // Add vault as operator
    bt_bill_client.add_operator(&vault_id);

    // Deploy repo market
    let repo_id = env.register_contract_wasm(None, repo_market::WASM);
    let repo_client = repo_market::Client::new(&env, &repo_id);
    repo_client.initialize(
        &admin,
        &treasury,
        &vault_id,
        &stablecoin,
        &bt_bill_token_id,
        &300i128,  // 3% haircut
        &200i128,  // 2% spread
    );

    // Add repo as operator
    bt_bill_client.add_operator(&repo_id);

    TestContext {
        env,
        admin,
        treasury,
        user1,
        user2,
        stablecoin,
        stablecoin_admin,
        bt_bill_token_id,
        vault_id,
        repo_id,
    }
}

#[test]
fn test_full_series_lifecycle() {
    let ctx = setup_test();
    let vault = bingo_vault::Client::new(&ctx.env, &ctx.vault_id);

    // Set ledger time
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    // Create series
    let series_id = 1u32;
    let issue_date = 1000u64;
    let maturity_date = 2000u64;
    let issue_price = 9_800_000i128; // 0.98
    let cap_par = 1_000_000i128 * SCALE;
    let user_cap_par = 100_000i128 * SCALE;

    vault.create_series(
        &series_id,
        &issue_date,
        &maturity_date,
        &issue_price,
        &cap_par,
        &user_cap_par,
    );

    // Activate series
    vault.activate_series(&series_id);

    let series = vault.get_series(&series_id);
    assert_eq!(series.0, series_id);
    assert_eq!(series.8, 1u32); // Status::Active
}

#[test]
fn test_subscribe_respects_series_cap() {
    let ctx = setup_test();
    let vault = bingo_vault::Client::new(&ctx.env, &ctx.vault_id);
    let stablecoin = token::Client::new(&ctx.env, &ctx.stablecoin);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    // Create and activate series with small cap
    let series_id = 1u32;
    let cap_par = 1000i128 * SCALE;
    let user_cap_par = 1000i128 * SCALE;

    vault.create_series(
        &series_id,
        &1000u64,
        &2000u64,
        &(9_800_000i128),
        &cap_par,
        &user_cap_par,
    );
    vault.activate_series(&series_id);

    // Try to subscribe more than cap
    let pay_amount = 1000i128 * SCALE; // This would mint ~1020 PAR at 0.98 price

    let result = vault.try_subscribe(&series_id, &pay_amount);
    // Should succeed up to cap
    // Let's try exact cap
    let exact_pay = (cap_par * 9_800_000) / PAR_UNIT;
    vault.subscribe(&series_id, &exact_pay);

    // Now try to subscribe more - should fail
    let result2 = vault.try_subscribe(&series_id, &(100i128 * SCALE));
    assert!(result2.is_err()); // ExceedsSeriesCap
}

#[test]
fn test_subscribe_respects_user_cap() {
    let ctx = setup_test();
    let vault = bingo_vault::Client::new(&ctx.env, &ctx.vault_id);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    let series_id = 1u32;
    let cap_par = 100_000i128 * SCALE;
    let user_cap_par = 1_000i128 * SCALE;

    vault.create_series(
        &series_id,
        &1000u64,
        &2000u64,
        &(9_800_000i128),
        &cap_par,
        &user_cap_par,
    );
    vault.activate_series(&series_id);

    // Subscribe up to user cap
    let pay_for_cap = (user_cap_par * 9_800_000) / PAR_UNIT;
    vault.subscribe(&series_id, &pay_for_cap);

    // Try to subscribe more - should fail
    let result = vault.try_subscribe(&series_id, &(100i128 * SCALE));
    assert!(result.is_err()); // ExceedsUserCap
}

#[test]
fn test_price_accretion() {
    let ctx = setup_test();
    let vault = bingo_vault::Client::new(&ctx.env, &ctx.vault_id);

    let series_id = 1u32;
    let issue_date = 1000u64;
    let maturity_date = 2000u64;
    let issue_price = 9_800_000i128; // 0.98

    vault.create_series(
        &series_id,
        &issue_date,
        &maturity_date,
        &issue_price,
        &(1_000_000i128 * SCALE),
        &(100_000i128 * SCALE),
    );

    // At issue date
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });
    let price_at_issue = vault.current_price(&series_id);
    assert_eq!(price_at_issue, issue_price);

    // At midpoint (should be ~0.99)
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1500,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });
    let price_at_mid = vault.current_price(&series_id);
    let expected_mid = issue_price + (PAR_UNIT - issue_price) / 2;
    assert_eq!(price_at_mid, expected_mid);

    // At maturity
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 2000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });
    let price_at_maturity = vault.current_price(&series_id);
    assert_eq!(price_at_maturity, PAR_UNIT);
}

#[test]
fn test_redeem_only_after_maturity() {
    let ctx = setup_test();
    let vault = bingo_vault::Client::new(&ctx.env, &ctx.vault_id);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    let series_id = 1u32;
    vault.create_series(
        &series_id,
        &1000u64,
        &2000u64,
        &(9_800_000i128),
        &(100_000i128 * SCALE),
        &(10_000i128 * SCALE),
    );
    vault.activate_series(&series_id);

    // Subscribe
    vault.subscribe(&series_id, &(1000i128 * SCALE));

    // Try to redeem before maturity - should fail
    let result = vault.try_redeem(&series_id, &(100i128 * SCALE));
    assert!(result.is_err()); // SeriesNotMatured

    // Move time to maturity
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 2000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    // Now redeem should work
    vault.redeem(&series_id, &(100i128 * SCALE));
}

#[test]
fn test_repo_open_and_close_happy_path() {
    let ctx = setup_test();
    let vault = bingo_vault::Client::new(&ctx.env, &ctx.vault_id);
    let repo = repo_market::Client::new(&ctx.env, &ctx.repo_id);
    let bt_bill = bt_bill_token::Client::new(&ctx.env, &ctx.bt_bill_token_id);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    // Create and activate series
    let series_id = 1u32;
    vault.create_series(
        &series_id,
        &1000u64,
        &3000u64,
        &(9_800_000i128),
        &(1_000_000i128 * SCALE),
        &(100_000i128 * SCALE),
    );
    vault.activate_series(&series_id);

    // User subscribes
    let subscribe_amount = 10_000i128 * SCALE;
    vault.subscribe(&series_id, &subscribe_amount);

    let user_balance = bt_bill.balance_of(&series_id, &ctx.user1);
    assert!(user_balance > 0);

    // Open repo
    let collateral_par = 5_000i128 * SCALE;
    let desired_cash = 4_500i128 * SCALE;
    let deadline = 2500u64;

    let position_id = repo.open_repo(&series_id, &collateral_par, &desired_cash, &deadline);
    assert_eq!(position_id, 1);

    // Close repo before deadline
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 2000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    repo.close_repo(&position_id);

    let position = repo.get_position(&position_id);
    assert_eq!(position.8, 1u32); // Status::Closed
}

#[test]
fn test_repo_default_path() {
    let ctx = setup_test();
    let vault = bingo_vault::Client::new(&ctx.env, &ctx.vault_id);
    let repo = repo_market::Client::new(&ctx.env, &ctx.repo_id);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    // Setup series and subscribe
    let series_id = 1u32;
    vault.create_series(
        &series_id,
        &1000u64,
        &5000u64,
        &(9_800_000i128),
        &(1_000_000i128 * SCALE),
        &(100_000i128 * SCALE),
    );
    vault.activate_series(&series_id);
    vault.subscribe(&series_id, &(10_000i128 * SCALE));

    // Open repo
    let collateral_par = 5_000i128 * SCALE;
    let desired_cash = 4_500i128 * SCALE;
    let deadline = 2000u64;
    let position_id = repo.open_repo(&series_id, &collateral_par, &desired_cash, &deadline);

    // Try to claim default before deadline - should fail
    let result = repo.try_claim_default(&position_id);
    assert!(result.is_err()); // DeadlineNotPassed

    // Move past deadline
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 2001,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    // Now claim default should work
    repo.claim_default(&position_id);

    let position = repo.get_position(&position_id);
    assert_eq!(position.8, 2u32); // Status::Defaulted
}

#[test]
fn test_repo_respects_haircut() {
    let ctx = setup_test();
    let vault = bingo_vault::Client::new(&ctx.env, &ctx.vault_id);
    let repo = repo_market::Client::new(&ctx.env, &ctx.repo_id);

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    let series_id = 1u32;
    vault.create_series(
        &series_id,
        &1000u64,
        &5000u64,
        &(10_000_000i128), // Price = 1.0 for simplicity
        &(1_000_000i128 * SCALE),
        &(100_000i128 * SCALE),
    );
    vault.activate_series(&series_id);
    vault.subscribe(&series_id, &(10_000i128 * SCALE));

    let collateral_par = 10_000i128 * SCALE;
    // With 3% haircut, max cash = 10000 * 0.97 = 9700
    let desired_cash = 9_700i128 * SCALE;

    let position_id = repo.open_repo(&series_id, &collateral_par, &desired_cash, &3000u64);
    assert_eq!(position_id, 1);

    // Try to borrow more than max - should fail
    let result = repo.try_open_repo(&series_id, &collateral_par, &(9_800i128 * SCALE), &3000u64);
    assert!(result.is_err()); // ExceedsMaxCash
}

#[test]
fn test_complete_integration_flow() {
    let ctx = setup_test();
    let vault = bingo_vault::Client::new(&ctx.env, &ctx.vault_id);
    let repo = repo_market::Client::new(&ctx.env, &ctx.repo_id);
    let bt_bill = bt_bill_token::Client::new(&ctx.env, &ctx.bt_bill_token_id);
    let stablecoin = token::Client::new(&ctx.env, &ctx.stablecoin);

    // Start at issue date
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    // 1. Create series
    let series_id = 1u32;
    vault.create_series(
        &series_id,
        &1000u64,
        &10000u64,
        &(9_500_000i128), // 0.95
        &(1_000_000i128 * SCALE),
        &(50_000i128 * SCALE),
    );

    // 2. Activate series
    vault.activate_series(&series_id);

    // 3. User1 subscribes
    let user1_initial_stable = stablecoin.balance(&ctx.user1);
    vault.subscribe(&series_id, &(10_000i128 * SCALE));
    let user1_bt_bills = bt_bill.balance_of(&series_id, &ctx.user1);
    assert!(user1_bt_bills > 10_000i128 * SCALE); // Got more than 1:1 due to discount

    // 4. User1 opens repo
    let collateral = 5_000i128 * SCALE;
    let cash_request = 4_000i128 * SCALE;
    let repo_id = repo.open_repo(&series_id, &collateral, &cash_request, &5000u64);

    // 5. Move time forward
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 6000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    // 6. Price should have accreted
    let mid_price = vault.current_price(&series_id);
    assert!(mid_price > 9_500_000i128);
    assert!(mid_price < PAR_UNIT);

    // 7. User2 subscribes at higher price
    vault.subscribe(&series_id, &(5_000i128 * SCALE));
    let user2_bt_bills = bt_bill.balance_of(&series_id, &ctx.user2);
    assert!(user2_bt_bills < user1_bt_bills); // Got less due to higher price

    // 8. Move to maturity
    ctx.env.ledger().set(LedgerInfo {
        timestamp: 10000,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });

    let maturity_price = vault.current_price(&series_id);
    assert_eq!(maturity_price, PAR_UNIT);

    // 9. User2 redeems
    let user2_balance_before = bt_bill.balance_of(&series_id, &ctx.user2);
    vault.redeem(&series_id, &user2_balance_before);
    let user2_balance_after = bt_bill.balance_of(&series_id, &ctx.user2);
    assert_eq!(user2_balance_after, 0);

    // 10. Repo defaulted (deadline was 5000, now it's 10000)
    repo.claim_default(&repo_id);
    let position = repo.get_position(&repo_id);
    assert_eq!(position.8, 2u32); // Defaulted
}
