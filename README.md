<<<<<<< HEAD
# BINGO Protocol - Tokenized T-Bills on Stellar

A Soroban-based protocol for tokenizing U.S. Treasury Bills on the Stellar network with deterministic pricing, anti-monopoly caps, and an integrated repo market.

## Features

- **Real-World Backed**: Treasury purchases T-Bills off-chain first, then creates on-chain Series with hard caps
- **Zero-Coupon Mechanics**: Linear price accretion from issue price to PAR (1.0) at maturity
- **No Oracle Dependency**: All pricing is deterministic and calculated on-chain
- **Anti-Monopoly**: Per-user caps prevent concentration of holdings
- **Repo Market**: Borrow stablecoin against T-Bill collateral without liquidations
- **Pausable**: Emergency pause mechanism for all contracts
- **Comprehensive Events**: All major actions emit events for off-chain indexing

## Architecture

The protocol consists of three smart contracts:

### 1. bt_bill_token
Token contract representing fractional ownership of T-Bills in PAR units.
- Single token contract with balances keyed by `(series_id, user)`
- Only authorized operators (vault, repo) can mint/burn
- Users can freely transfer tokens

### 2. bingo_vault
Core protocol managing Series lifecycle and user subscriptions.
- Creates and manages T-Bill Series
- Handles subscriptions (mint bT-Bills) and redemptions (burn bT-Bills)
- Calculates deterministic price accretion
- Enforces series caps and per-user caps

### 3. repo_market
Single-lender repo market for borrowing against T-Bill collateral.
- Treasury provides stablecoin liquidity
- Borrowers deposit bT-Bills, receive stablecoin (with haircut)
- Binary outcome: repay or default (no liquidations)

## Constants

```rust
SCALE = 10_000_000           // 7 decimals for fixed-point
PAR_UNIT = 1 * SCALE         // 1.0000000
BASIS_POINTS = 10_000        // For percentage calculations
```

## Price Accretion Formula

```
current_price(t) = min(PAR_UNIT, issue_price + (PAR_UNIT - issue_price) × (t - issue_date) / (maturity - issue_date))
```

Example:
- Issue price: 0.98 ($0.98 per $1 PAR)
- Issue date: Day 0
- Maturity: Day 365
- Price at Day 180: 0.99 (halfway to PAR)
- Price at Day 365+: 1.00 (PAR)

## Building

```bash
# Build all contracts
cargo build --target wasm32-unknown-unknown --release

# Optimize WASM (optional)
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/bt_bill_token.wasm \
  --wasm-out target/wasm32-unknown-unknown/release/bt_bill_token_optimized.wasm
```

## Testing

```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test integration_test
```

## Deployment

### 1. Deploy bt_bill_token

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/bt_bill_token.wasm \
  --source ADMIN_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"
```

Save the contract ID as `BT_BILL_TOKEN_ID`.

### 2. Initialize bt_bill_token

```bash
soroban contract invoke \
  --id $BT_BILL_TOKEN_ID \
  --source ADMIN_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- initialize \
  --admin ADMIN_ADDRESS
```

### 3. Deploy bingo_vault

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/bingo_vault.wasm \
  --source ADMIN_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"
```

Save as `VAULT_ID`.

### 4. Initialize bingo_vault

```bash
soroban contract invoke \
  --id $VAULT_ID \
  --source ADMIN_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- initialize \
  --admin ADMIN_ADDRESS \
  --treasury TREASURY_ADDRESS \
  --stablecoin USDC_CONTRACT_ID \
  --bt_bill_token $BT_BILL_TOKEN_ID
```

### 5. Add vault as operator

```bash
soroban contract invoke \
  --id $BT_BILL_TOKEN_ID \
  --source ADMIN_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- add_operator \
  --operator $VAULT_ID
```

### 6. Deploy repo_market

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/repo_market.wasm \
  --source ADMIN_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"
```

Save as `REPO_ID`.

### 7. Initialize repo_market

```bash
soroban contract invoke \
  --id $REPO_ID \
  --source ADMIN_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- initialize \
  --admin ADMIN_ADDRESS \
  --treasury TREASURY_ADDRESS \
  --vault $VAULT_ID \
  --stablecoin USDC_CONTRACT_ID \
  --bt_bill_token $BT_BILL_TOKEN_ID \
  --haircut_bps 300 \
  --spread_bps 200
```

### 8. Add repo as operator

```bash
soroban contract invoke \
  --id $BT_BILL_TOKEN_ID \
  --source ADMIN_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- add_operator \
  --operator $REPO_ID
```

## Usage Examples

### Create a Series

```bash
soroban contract invoke \
  --id $VAULT_ID \
  --source TREASURY_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- create_series \
  --series_id 1 \
  --issue_date 1704067200 \
  --maturity_date 1735689600 \
  --issue_price 9800000 \
  --cap_par 100000000000000 \
  --user_cap_par 10000000000000
```

Parameters:
- `series_id`: Unique identifier (1, 2, 3...)
- `issue_date`: Unix timestamp
- `maturity_date`: Unix timestamp (must be > issue_date)
- `issue_price`: Price in SCALE units (9800000 = 0.98)
- `cap_par`: Max PAR units mintable (100000000000000 = 10,000,000)
- `user_cap_par`: Max PAR per user (10000000000000 = 1,000,000)

### Activate Series

```bash
soroban contract invoke \
  --id $VAULT_ID \
  --source TREASURY_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- activate_series \
  --series_id 1
```

### Subscribe (User)

```bash
# First approve stablecoin transfer
soroban contract invoke \
  --id $USDC_ID \
  --source USER_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- approve \
  --from USER_ADDRESS \
  --spender $VAULT_ID \
  --amount 10000000000

# Then subscribe
soroban contract invoke \
  --id $VAULT_ID \
  --source USER_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- subscribe \
  --series_id 1 \
  --pay_amount 10000000000
```

### Check Balance

```bash
soroban contract invoke \
  --id $BT_BILL_TOKEN_ID \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- balance_of \
  --series_id 1 \
  --user USER_ADDRESS
```

### Redeem (at Maturity)

```bash
soroban contract invoke \
  --id $VAULT_ID \
  --source USER_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- redeem \
  --series_id 1 \
  --bt_bill_amount 5000000000
```

### Open Repo

```bash
soroban contract invoke \
  --id $REPO_ID \
  --source USER_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- open_repo \
  --series_id 1 \
  --collateral_par 10000000000 \
  --desired_cash_out 9000000000 \
  --deadline 1720000000
```

### Close Repo

```bash
soroban contract invoke \
  --id $REPO_ID \
  --source USER_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- close_repo \
  --position_id 1
```

### Claim Default (Treasury)

```bash
soroban contract invoke \
  --id $REPO_ID \
  --source TREASURY_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- claim_default \
  --position_id 1
```

## Events

All contracts emit events for major actions:

### bt_bill_token Events
- `mint`: `(series_id, to, amount)`
- `burn`: `(series_id, from, amount)`
- `transfer`: `(series_id, from, to, amount)`

### bingo_vault Events
- `series_created`: `(series_id, issue_date, maturity_date, issue_price, cap_par)`
- `series_activated`: `(series_id)`
- `subscribed`: `(series_id, user, pay_amount, minted_par, price)`
- `redeemed`: `(series_id, user, bt_bill_amount, payout)`

### repo_market Events
- `repo_opened`: `(position_id, borrower, series_id, collateral_par, cash_out, deadline)`
- `repo_closed`: `(position_id, borrower)`
- `repo_defaulted`: `(position_id, borrower, collateral_par)`

## Security Considerations

1. **Access Control**: Admin and Treasury roles are strictly enforced
2. **Reentrancy**: All state updates happen before external calls
3. **Integer Overflow**: Uses checked arithmetic throughout
4. **Authorization**: All user actions require explicit auth
5. **Pausable**: Emergency pause for all contracts
6. **Operator Allowlist**: Only authorized contracts can mint/burn tokens

## Error Codes

### Common Errors
- `AlreadyInitialized (1)`: Contract already initialized
- `NotInitialized (2)`: Contract not yet initialized
- `Unauthorized (3)`: Caller not authorized
- `InvalidAmount (5)`: Invalid amount (≤0 or overflow)
- `ContractPaused (14)`: Contract is paused

### Vault Errors
- `SeriesNotFound (4)`: Series doesn't exist
- `SeriesNotActive (5)`: Series not in Active status
- `SeriesNotMatured (6)`: Series hasn't reached maturity
- `ExceedsSeriesCap (8)`: Would exceed series cap
- `ExceedsUserCap (9)`: Would exceed user cap

### Repo Errors
- `PositionNotFound (4)`: Repo position doesn't exist
- `DeadlineNotPassed (7)`: Cannot claim before deadline
- `DeadlinePassed (8)`: Cannot close after deadline
- `ExceedsMaxCash (10)`: Requested cash exceeds LTV limit

## Gas Optimization

- Fixed-point arithmetic (7 decimals) avoids expensive floating-point ops
- Compact data structures minimize storage costs
- Batch operations where possible
- Events use indexed fields for efficient querying

## Future Enhancements

1. **Multi-Lender Repo**: Support multiple lenders in repo market
2. **Secondary Market**: P2P trading of bT-Bills
3. **Series Rolling**: Automatic rollover at maturity
4. **Yield Aggregation**: Composite products across series
5. **Cross-Chain Bridge**: Bridge bT-Bills to other chains

## License

MIT

