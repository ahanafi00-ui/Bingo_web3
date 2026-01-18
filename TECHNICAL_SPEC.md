# BINGO Protocol Technical Specification

## 1. Overview

BINGO (Blockchain-Integrated Note for Government Obligations) is a Soroban smart contract protocol for tokenizing U.S. Treasury Bills on the Stellar blockchain. The protocol provides:

- **Real-world backed T-Bill exposure** with on-chain representation
- **Deterministic pricing** via linear accretion without oracles
- **Capital efficiency** through an integrated repo market
- **Anti-monopoly controls** via per-user caps

## 2. Mathematical Model

### 2.1 Fixed-Point Arithmetic

All amounts use i128 fixed-point arithmetic with 7 decimal precision:

```
SCALE = 10,000,000
PAR_UNIT = 1 × SCALE = 10,000,000 (represents 1.0)
```

Example conversions:
- 0.98 = 9,800,000
- 1,000 = 10,000,000,000
- 1,000,000.50 = 10,000,005,000,000

### 2.2 Price Accretion Formula

T-Bills are zero-coupon instruments that accrete from issue price to PAR at maturity:

```
price(t) = min(PAR_UNIT, issue_price + (PAR_UNIT - issue_price) × (t - issue_date) / (maturity - issue_date))
```

Where:
- `t` = current timestamp
- `issue_price` = initial discounted price (e.g., 0.98)
- `PAR_UNIT` = 1.0 (maturity value)
- `issue_date` = series issue timestamp
- `maturity_date` = series maturity timestamp

**Properties:**
- Linear accretion: constant daily yield
- Always ≤ PAR_UNIT (caps at maturity)
- No oracle dependency: purely time-based
- Deterministic: same inputs always produce same output

**Example:**
```
Issue: Day 0 at 0.95 ($950 per $1,000 PAR)
Maturity: Day 365
Day 180: 0.95 + (1.0 - 0.95) × (180/365) = 0.9747
Day 365: 1.0 (PAR)
```

### 2.3 Subscription Calculation

When a user subscribes, they receive PAR units proportional to their payment:

```
minted_par = pay_amount × PAR_UNIT / current_price(t)
```

**Example at 0.98 price:**
```
User pays: 1,000 USDC
Current price: 0.98
Minted PAR: 1,000 × 1.0 / 0.98 = 1,020.41 PAR
```

At maturity, 1,020.41 PAR redeems for 1,020.41 USDC (profit: 20.41 USDC)

### 2.4 Repo Loan-to-Value

Repo market calculates max borrowing with haircut:

```
collateral_value = collateral_par × current_price / PAR_UNIT
max_cash = collateral_value × (BASIS_POINTS - haircut_bps) / BASIS_POINTS
repurchase_amount = cash_out × (BASIS_POINTS + spread_bps) / BASIS_POINTS
```

**Example with 3% haircut, 2% spread:**
```
Collateral: 10,000 PAR
Current price: 0.99
Collateral value: 10,000 × 0.99 = 9,900 USDC
Max cash (3% haircut): 9,900 × 0.97 = 9,603 USDC
Borrower requests: 9,500 USDC
Repurchase amount (2% spread): 9,500 × 1.02 = 9,690 USDC
```

## 3. Data Structures

### 3.1 Series

```rust
struct Series {
    series_id: u32,           // Unique identifier
    issue_date: u64,          // Unix timestamp
    maturity_date: u64,       // Unix timestamp (must be > issue_date)
    par_unit: i128,           // Always 1 × SCALE
    issue_price: i128,        // Discounted price (e.g., 0.98 × SCALE)
    cap_par: i128,            // Max PAR mintable for series
    minted_par: i128,         // Current PAR minted
    user_cap_par: i128,       // Max PAR per user
    status: SeriesStatus,     // UPCOMING | ACTIVE | MATURED | CLOSED
}

enum SeriesStatus {
    Upcoming = 0,  // Created but not yet active
    Active = 1,    // Open for subscriptions
    Matured = 2,   // Past maturity, redemptions allowed
    Closed = 3,    // Series ended
}
```

**Invariants:**
- `maturity_date > issue_date`
- `0 < issue_price ≤ PAR_UNIT`
- `0 < user_cap_par ≤ cap_par`
- `minted_par ≤ cap_par` (enforced on every subscription)

### 3.2 User Position

```rust
struct UserPosition {
    subscribed_par: i128,  // Total PAR subscribed by user in this series
}
```

**Storage key:** `(series_id, user_address)`

**Invariant:**
- `user.subscribed_par ≤ series.user_cap_par` (enforced on every subscription)

### 3.3 Repo Position

```rust
struct RepoPosition {
    id: u64,                   // Unique position ID
    borrower: Address,         // Borrower address
    series_id: u32,            // Collateral series
    collateral_par: i128,      // PAR units locked as collateral
    cash_out: i128,            // Stablecoin borrowed
    repurchase_amount: i128,   // Amount to repay (principal + fee)
    start_time: u64,           // Position open timestamp
    deadline: u64,             // Repayment deadline
    status: RepoStatus,        // OPEN | CLOSED | DEFAULTED
}

enum RepoStatus {
    Open = 0,       // Active position
    Closed = 1,     // Repaid successfully
    Defaulted = 2,  // Deadline passed, lender claimed collateral
}
```

**Invariants:**
- `deadline ≤ series.maturity_date`
- `cash_out ≤ max_cash` (calculated with haircut)
- Status transitions: `OPEN → CLOSED` (repay) or `OPEN → DEFAULTED` (claim)

## 4. State Transitions

### 4.1 Series Lifecycle

```
┌──────────┐  create_series   ┌──────────┐  activate_series  ┌────────┐
│          │ ───────────────→  │          │ ─────────────────→ │        │
│   NULL   │                   │ UPCOMING │                    │ ACTIVE │
│          │                   │          │                    │        │
└──────────┘                   └──────────┘                    └────────┘
                                                                   │
                                                                   │ time ≥ maturity_date
                                                                   ↓
                                                               ┌─────────┐
                                                               │         │
                                                               │ MATURED │
                                                               │         │
                                                               └─────────┘
```

### 4.2 Subscription Flow

```
User                    Vault                    Token                   Stablecoin
 │                       │                         │                          │
 │  subscribe(1000)      │                         │                          │
 ├──────────────────────→│                         │                          │
 │                       │  transfer(user→vault)   │                          │
 │                       ├─────────────────────────┼─────────────────────────→│
 │                       │                         │                          │
 │                       │  mint(series, user, PAR)│                          │
 │                       ├────────────────────────→│                          │
 │                       │                         │                          │
 │  ← success            │                         │                          │
 │←──────────────────────┤                         │                          │
```

**Steps:**
1. User calls `vault.subscribe(series_id, pay_amount)`
2. Vault verifies series is ACTIVE
3. Vault calculates `current_price(t)`
4. Vault computes `minted_par = pay_amount × PAR / price`
5. Vault checks caps: `series.minted_par + minted_par ≤ cap_par`
6. Vault checks user cap: `user.subscribed_par + minted_par ≤ user_cap_par`
7. Vault transfers stablecoin from user to vault
8. Vault calls `token.mint(series_id, user, minted_par)`
9. Vault updates `series.minted_par` and `user.subscribed_par`
10. Vault emits `Subscribed` event

### 4.3 Redemption Flow

```
User                    Vault                    Token                   Stablecoin
 │                       │                         │                          │
 │  redeem(1000 PAR)     │                         │                          │
 ├──────────────────────→│                         │                          │
 │                       │  burn(series, user, PAR)│                          │
 │                       ├────────────────────────→│                          │
 │                       │                         │                          │
 │                       │  transfer(vault→user)   │                          │
 │                       ├─────────────────────────┼─────────────────────────→│
 │                       │                         │                          │
 │  ← success            │                         │                          │
 │←──────────────────────┤                         │                          │
```

**Preconditions:**
- `current_time ≥ maturity_date` OR `series.status == MATURED`
- User has sufficient bT-Bill balance

**Steps:**
1. User calls `vault.redeem(series_id, bt_bill_amount)`
2. Vault verifies maturity condition
3. Vault calls `token.burn(series_id, user, bt_bill_amount)`
4. Vault transfers stablecoin equal to PAR value (1:1)
5. Vault emits `Redeemed` event

### 4.4 Repo Open Flow

```
Borrower              Repo                Token               Treasury
   │                   │                    │                    │
   │  open_repo()      │                    │                    │
   ├──────────────────→│                    │                    │
   │                   │  get_price()       │                    │
   │                   ├ ─ ─ ─ ─ → (vault) │                    │
   │                   │                    │                    │
   │                   │  transfer(b→repo)  │                    │
   │                   ├───────────────────→│                    │
   │                   │                    │                    │
   │                   │  transfer(t→b, $)  │                    │
   │                   ├────────────────────┼───────────────────→│
   │                   │                    │                    │
   │  ← position_id    │                    │                    │
   │←──────────────────┤                    │                    │
```

### 4.5 Repo Close Flow (Happy Path)

```
Borrower              Repo                Token               Treasury
   │                   │                    │                    │
   │  close_repo(id)   │                    │                    │
   ├──────────────────→│                    │                    │
   │                   │  transfer(b→t, $+fee)                   │
   │                   ├────────────────────┼───────────────────→│
   │                   │                    │                    │
   │                   │  transfer(repo→b)  │                    │
   │                   ├───────────────────→│                    │
   │                   │                    │                    │
   │  ← success        │                    │                    │
   │←──────────────────┤                    │                    │
```

### 4.6 Repo Default Flow

```
Treasury              Repo                Token
   │                   │                    │
   │  claim_default(id)│                    │
   ├──────────────────→│                    │
   │                   │  (verify deadline) │
   │                   │  transfer(repo→t)  │
   │                   ├───────────────────→│
   │                   │                    │
   │  ← success        │                    │
   │←──────────────────┤                    │
```

## 5. Security Model

### 5.1 Access Control

**Admin Role:**
- Initialize contracts
- Add/remove operators
- Pause/unpause contracts
- Update repo parameters (haircut, spread)

**Treasury Role:**
- Create series
- Activate series
- Provide repo liquidity
- Claim defaulted collateral

**User Role:**
- Subscribe to series
- Redeem at maturity
- Transfer bT-Bills
- Open/close repo positions

### 5.2 Authorization Patterns

All user-facing functions use `require_auth()`:

```rust
user.require_auth();  // Verifies signature from user
```

Admin/Treasury functions check role:

```rust
let admin: Address = env.storage().instance().get(&DataKey::Admin)?;
admin.require_auth();
```

### 5.3 Reentrancy Protection

Pattern: State updates before external calls

```rust
// ✅ SAFE: Update state first
series.minted_par += minted_par;
env.storage().instance().set(&DataKey::Series(id), &series);

// Then make external call
stablecoin.transfer(&user, &vault, &amount);
```

```rust
// ❌ UNSAFE: External call before state update
stablecoin.transfer(&user, &vault, &amount);
series.minted_par += minted_par;  // Vulnerable to reentrancy
```

### 5.4 Integer Overflow Protection

All arithmetic uses checked operations:

```rust
let new_balance = current_balance
    .checked_add(amount)
    .ok_or(Error::InvalidAmount)?;
```

Prevents:
- Overflow attacks
- Underflow in subtractions
- Division by zero

### 5.5 Operator Allowlist

Only authorized contracts can mint/burn tokens:

```rust
// In bt_bill_token
pub fn add_operator(env: Env, operator: Address) -> Result<(), Error> {
    let admin: Address = env.storage().instance().get(&DataKey::Admin)?;
    admin.require_auth();
    
    env.storage().instance().set(&DataKey::Operators(operator), &true);
    Ok(())
}
```

This ensures:
- Only vault can mint during subscriptions
- Only vault can burn during redemptions
- Only repo can escrow collateral
- No unauthorized token creation

## 6. Economic Mechanisms

### 6.1 Yield Calculation

**Implicit Yield:**
```
Yield = (PAR - issue_price) / issue_price
```

Example:
- Issue price: 0.98
- PAR: 1.00
- Yield: (1.00 - 0.98) / 0.98 = 2.04%

**Annualized Yield:**
```
Annual Yield = Yield × (365 / days_to_maturity)
```

Example for 90-day T-Bill:
- Yield: 2.04%
- Annualized: 2.04% × (365/90) = 8.27%

### 6.2 Anti-Monopoly Caps

**Two-tier cap system:**

1. **Series Cap:** Total PAR mintable across all users
   - Prevents over-issuance beyond off-chain backing
   - Hard limit enforced on-chain

2. **User Cap:** Max PAR per individual user
   - Prevents concentration
   - Promotes decentralization
   - Configurable per series

**Example:**
```
Series cap: 100M PAR
User cap: 1M PAR
Max users at cap: 100
Minimum users required to fill series: 100
```

### 6.3 Repo Economics

**Haircut Protection:**
- Lender protected from price volatility
- Borrower gets immediate liquidity at discount
- Example: 3% haircut means 97% LTV

**Spread Income:**
- Compensates lender for capital lockup
- Compensates for default risk
- Example: 2% spread on 9,500 USDC = 190 USDC fee

**No Liquidations:**
- Binary outcome: repay or default
- No oracle dependency
- Deadline enforcement is deterministic

## 7. Gas Optimization

### 7.1 Storage Efficiency

**Compact types:**
```rust
u32 for IDs (vs u64)           // 50% less storage
i128 for amounts (sufficient)   // vs i256
```

**Lazy initialization:**
```rust
// UserPosition not created until first subscription
// Saves storage for non-participants
```

### 7.2 Computation Efficiency

**Fixed-point arithmetic:**
- No floating-point operations
- Integer-only math (faster)
- Predictable gas costs

**Minimal external calls:**
```rust
// Single price calculation vs multiple oracle queries
// O(1) vs O(n) complexity
```

### 7.3 Event Design

**Indexed fields:**
```rust
env.events().publish(
    (Symbol::new(&env, "subscribed"), series_id),  // Indexed
    SubscribedEvent { ... }                        // Data
);
```

Benefits:
- Efficient off-chain indexing
- Fast filtering by series_id
- Reduced on-chain query costs

## 8. Testing Strategy

### 8.1 Unit Tests

**Coverage:**
- Contract initialization
- State transitions
- Error conditions
- Edge cases (zero amounts, exact caps, boundary timestamps)

### 8.2 Integration Tests

**Scenarios:**
- Full lifecycle: create → activate → subscribe → redeem
- Repo happy path: open → close
- Repo default path: open → timeout → claim
- Price accretion verification
- Multi-user interactions

### 8.3 Invariant Tests

**Critical invariants:**
- `series.minted_par ≤ series.cap_par`
- `user.subscribed_par ≤ series.user_cap_par`
- `price(t) ≤ PAR_UNIT`
- `Total minted = Sum of user balances`

## 9. Future Enhancements

### 9.1 Multi-Lender Repo

```rust
struct RepoOffer {
    lender: Address,
    series_id: u32,
    available_liquidity: i128,
    haircut_bps: i128,
    spread_bps: i128,
}
```

Enables:
- Competitive lending rates
- Distributed liquidity provision
- Lender yield opportunities

### 9.2 Secondary Market

```rust
struct Order {
    maker: Address,
    series_id: u32,
    amount: i128,
    price: i128,
    side: OrderSide,  // BUY | SELL
}
```

Enables:
- Early exit before maturity
- Price discovery
- Liquidity for holders

### 9.3 Series Rolling

```rust
fn roll_series(
    from_series: u32,
    to_series: u32,
    amount: i128
) -> Result<(), Error>;
```

Enables:
- Automatic reinvestment
- Continuous exposure
- Reduced transaction costs

## 10. References

- Soroban Documentation: https://docs.stellar.org/docs/soroban
- U.S. Treasury: https://www.treasurydirect.gov/
- Stellar Asset Contract: https://docs.stellar.org/docs/tokens/stellar-asset-contract
- Fixed-Point Arithmetic: https://en.wikipedia.org/wiki/Fixed-point_arithmetic
