# BINGO Protocol Architecture

## Overview

BINGO is a tokenized T-Bill protocol on Stellar that enables on-chain representation of real-world U.S. Treasury Bills. The protocol consists of three main smart contracts working together to provide secure, transparent, and capital-efficient T-Bill exposure.

## Core Design Principles

1. **Real-World Backing**: Treasury purchases T-Bills off-chain first, then creates on-chain Series with hard caps matching the underlying par amount
2. **Zero-Coupon Mechanics**: Each Series represents a zero-coupon instrument that accretes from issue price to PAR (1.0) at maturity
3. **Anti-Monopoly**: Per-user caps prevent concentration of holdings
4. **No Oracle Dependency**: All pricing is deterministic based on linear accretion
5. **Capital Efficiency**: Optional repo market allows borrowing against T-Bill collateral without liquidations

## Contract Architecture

### 1. bingo_vault (Core Protocol)
**Purpose**: Manages Series lifecycle, user subscriptions, redemptions, and accounting

**Key Functions**:
- `create_series`: Admin creates a new T-Bill series with cap and pricing parameters
- `activate_series`: Makes series available for subscriptions
- `subscribe`: Users deposit stablecoin to mint bT-Bills at current price
- `redeem`: Users burn bT-Bills at maturity to receive PAR value
- `current_price`: Deterministic linear accretion from issue_price to PAR

**Invariants**:
- `series.minted_par ≤ series.cap_par` (global series cap)
- `user.subscribed_par ≤ series.user_cap_par` (per-user cap)
- Price accretes linearly: `price(t) = issue_price + (PAR - issue_price) × (t - issue_date) / (maturity - issue_date)`
- Redemption only at/after maturity

### 2. bt_bill_token (Token Contract)
**Purpose**: Represents fractional ownership of T-Bills in PAR units

**Design Choice**: Single token contract with balances keyed by `(series_id, user)`

**Rationale**:
- **Flexibility**: One contract handles all series without deploying new contracts
- **Gas Efficiency**: Fewer contract deployments, centralized storage
- **Simplicity**: Easier to manage allowlist of authorized operators (vault + repo)
- **Cross-Series Operations**: Enables future features like series rolling

**Key Functions**:
- `mint`: Only vault/repo can mint tokens (authorized operators)
- `burn`: Only vault/repo can burn tokens
- `transfer`: Users can transfer tokens with auth
- `balance_of`: Query balance for (series_id, user)

**Access Control**: Operator allowlist ensures only vault and repo contracts can mint/burn

### 3. repo_market (Collateralized Lending)
**Purpose**: Single-lender MVP where Treasury provides liquidity against bT-Bill collateral

**Key Features**:
- **No Liquidations**: Binary outcome - repay or default
- **No Oracles**: Uses vault's deterministic pricing
- **Treasury as Lender**: Simplified single-lender model
- **Time-Based**: Repo deadline must be ≤ series maturity

**Repo Mechanics**:
1. **Open**: Borrower deposits bT-Bills, receives stablecoin (haircut applied)
2. **Close**: Borrower repays principal + fee before deadline, receives collateral
3. **Default**: If deadline passes, lender keeps collateral and can redeem at maturity

**Haircut Example**: 3% haircut means borrower gets 97% of mark-to-market value in stablecoin

## Data Flow

### Subscription Flow
```
User → [stablecoin] → Vault
Vault → [calculate PAR units] → bt_bill_token.mint()
bt_bill_token → [bT-Bills] → User
```

### Redemption Flow (at Maturity)
```
User → [bT-Bills] → Vault
Vault → bt_bill_token.burn()
Vault → [stablecoin (PAR value)] → User
```

### Repo Flow
```
Open:
  Borrower → [bT-Bills] → repo_market (escrow)
  Treasury → [stablecoin] → Borrower
Close:
  Borrower → [stablecoin + fee] → Treasury
  repo_market → [bT-Bills] → Borrower
Default:
  repo_market → [bT-Bills] → Treasury (keep collateral)
```

## Mathematical Model

### Constants
- `SCALE = 10_000_000` (7 decimals for fixed-point arithmetic)
- `PAR_UNIT = 1 * SCALE` (1.0000000)
- `BASIS_POINTS = 10_000`

### Price Accretion
```
current_price(t) = min(PAR_UNIT, issue_price + (PAR_UNIT - issue_price) × (t - issue_date) / (maturity - issue_date))
```

### Subscription Calculation
```
minted_par = pay_amount × PAR_UNIT / current_price(t)
```

### Repo Loan-to-Value
```
max_cash = collateral_par × current_price / PAR_UNIT × (1 - haircut)
repurchase_amount = cash_out × (1 + spread)
```

## Security Features

1. **Role-Based Access Control**: Admin/Treasury roles for privileged operations
2. **Reentrancy Protection**: Single-entry pattern with state updates before external calls
3. **Integer Overflow Protection**: Using i128 with checked arithmetic
4. **Authorization**: All user actions require `require_auth()`
5. **Operator Allowlist**: Only authorized contracts can mint/burn tokens
6. **Event Logging**: Comprehensive events for all state changes
7. **Pausable**: Emergency pause mechanism

## Error Handling

Exhaustive error enums for clear failure modes:
- `AlreadyInitialized`
- `NotInitialized`
- `SeriesNotFound`
- `SeriesNotActive`
- `SeriesNotMatured`
- `ExceedsSeriesCap`
- `ExceedsUserCap`
- `InsufficientBalance`
- `InvalidAmount`
- `DeadlineNotPassed`
- `DeadlinePassed`
- `Unauthorized`
- `ContractPaused`

## Events

All major actions emit events:
- `SeriesCreated`
- `SeriesActivated`
- `Subscribed`
- `Redeemed`
- `RepoOpened`
- `RepoClosed`
- `RepoDefaulted`
- `Transfer` (token)

## Upgrade Path & Future Features

1. **Multi-Lender Repo**: Extend repo market to support multiple lenders
2. **Secondary Market**: Enable peer-to-peer trading of bT-Bills
3. **Series Rolling**: Automatically roll maturing positions into new series
4. **Yield Aggregation**: Composite products across multiple series
5. **Cross-Chain Bridge**: Bridge bT-Bills to other chains

## Testing Strategy

Comprehensive unit tests cover:
1. Series lifecycle (create → activate → mature)
2. Subscription caps (series-level and user-level)
3. Price accretion mechanics
4. Redemption timing and calculations
5. Repo happy path (open → close)
6. Repo default path (deadline expiry)
7. Access control (unauthorized attempts)
8. Edge cases (zero amounts, exact caps, boundary conditions)

## Deployment Order

1. Deploy `bt_bill_token` contract
2. Deploy `bingo_vault` contract (pass token address)
3. Deploy `repo_market` contract (pass vault and token addresses)
4. Configure operator allowlist in token contract
5. Initialize vault with admin/treasury addresses
6. Initialize repo market with parameters

## Gas Optimization Notes

- Storage minimized by using compact data types
- Batch operations where possible
- Events use indexed fields for efficient querying
- Fixed-point arithmetic avoids expensive floating-point operations
