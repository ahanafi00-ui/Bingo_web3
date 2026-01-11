# BINGO MVP Technical Specification

## Overview

Smart contract MVP untuk BINGO Protocol yang implement core functionality:
- Obligation issuance dengan parameters
- User subscription dengan cap limits
- Time-weighted yield calculation
- KYC/AML verification
- Redemption at maturity

## Contract Architecture

### Storage Model

#### Instance Storage (Singleton)
- `Admin`: Address - contract administrator
- `NextSeriesId`: u32 - auto-incrementing series counter

#### Persistent Storage (Multi-entry)
- `Series(series_id)`: Series struct
- `UserPosition(series_id, user)`: UserPosition struct  
- `KYCVerified(user)`: bool

### Data Structures

```rust
struct Series {
    id: u32,
    par_value: i128,           // Face value scaled by 1e7
    subscription_price: i128,  // Initial price scaled by 1e7
    issue_time: u64,           // Unix timestamp
    maturity_time: u64,        // Unix timestamp
    max_cap: i128,             // Total subscription cap (shares)
    per_user_cap: i128,        // Per-user cap (shares)
    total_subscribed: i128,    // Current total (shares)
    status: SeriesStatus,
    usdc_token: Address,       // USDC contract address
}

enum SeriesStatus {
    Active,    // Accepting subscriptions
    Matured,   // Past maturity, awaiting settlement
    Settled,   // Settlement deposited, can redeem
}

struct UserPosition {
    shares: i128,       // Number of shares owned
    entry_index: i128,  // Index at subscription time
}
```

## Mathematical Model

### Precision & Scaling

All monetary values use **1e7 scaling** for precision:
- `SCALE = 10_000_000`
- Example: $1.00 = 1,000,000 (internal)
- Example: $0.98 = 980,000 (internal)

### Index Calculation

The index represents the current price per SCALE shares:

```
current_index = subscription_price + accrued_yield

Where:
accrued_yield = (par_value - subscription_price) × (elapsed / duration)
elapsed = current_time - issue_time
duration = maturity_time - issue_time
```

**At issue**: `index = subscription_price`
**At maturity**: `index = par_value`
**Between**: Linear interpolation

### Share Calculation

When user subscribes with USDC amount:

```
shares = (usdc_amount × SCALE) / current_index
```

This ensures:
- Early subscribers get more shares (lower index)
- Late subscribers get fewer shares (higher index)
- Fair time-weighted allocation

### Position Value

User's current position value:

```
value = (shares × current_index) / SCALE
```

### Yield Earned

User's unrealized yield at any time:

```
yield = value - cost_basis

Where:
cost_basis = (shares × entry_index) / SCALE
```

## Function Specifications

### Admin Functions

#### `initialize(admin: Address)`
- **Auth**: None (first-time only)
- **Pre**: Contract not initialized
- **Post**: Admin set, NextSeriesId = 0
- **Reverts**: If already initialized

#### `issue_series(...) -> u32`
- **Auth**: Admin
- **Pre**: 
  - subscription_price < par_value
  - maturity_time > current_time
  - max_cap > 0
  - per_user_cap > 0
- **Post**: 
  - New series created with status Active
  - NextSeriesId incremented
  - Returns series_id
- **Reverts**: Invalid parameters

#### `verify_kyc(user: Address)`
- **Auth**: Admin
- **Post**: User marked as KYC verified
- **Reverts**: None

#### `revoke_kyc(user: Address)`
- **Auth**: Admin
- **Post**: User marked as not KYC verified
- **Reverts**: None

#### `settle_series(series_id, usdc_amount, admin)`
- **Auth**: Admin
- **Pre**:
  - Series exists
  - current_time >= maturity_time
  - usdc_amount >= required_settlement
- **Post**:
  - USDC transferred from admin to contract
  - Series status = Settled
- **Reverts**: Insufficient amount, not matured

### User Functions

#### `subscribe(series_id, usdc_amount, user) -> i128`
- **Auth**: User
- **Pre**:
  - User is KYC verified
  - Series status = Active
  - current_time < maturity_time
  - total_subscribed + new_shares <= max_cap
  - user_shares + new_shares <= per_user_cap
- **Post**:
  - USDC transferred from user to contract
  - Shares minted to user
  - total_subscribed incremented
  - Returns shares minted
- **Reverts**: Cap exceeded, not KYC, matured

**Share calculation logic**:
```rust
current_index = calculate_index(series)
shares = (usdc_amount × SCALE) / current_index

// If first subscription
if existing_position.shares == 0:
    entry_index = current_index
else:
    entry_index = existing_position.entry_index (unchanged)

new_position = UserPosition {
    shares: existing_position.shares + shares,
    entry_index: entry_index
}
```

#### `redeem(series_id, user) -> i128`
- **Auth**: User
- **Pre**:
  - User is KYC verified
  - Series status = Settled
  - current_time >= maturity_time
  - User has position
- **Post**:
  - USDC transferred from contract to user
  - User position deleted
  - Returns USDC amount
- **Reverts**: Not matured, no position

**Redemption calculation**:
```rust
redemption_value = (user_shares × par_value) / SCALE
```

### View Functions

#### `get_series(series_id) -> Series`
Returns series details.

#### `get_user_position(series_id, user) -> UserPosition`
Returns user's position or default (0 shares).

#### `get_position_value(series_id, user) -> i128`
Calculates current value based on accrued yield:
```rust
current_index = calculate_index(series)
value = (user_shares × current_index) / SCALE
```

#### `is_kyc_verified(user) -> bool`
Returns KYC status.

## Security Properties

### Invariants

1. **Supply Conservation**:
   ```
   total_subscribed <= max_cap
   ```

2. **User Cap Enforcement**:
   ```
   ∀ user: user_position.shares <= per_user_cap
   ```

3. **Index Monotonicity**:
   ```
   ∀ t1 < t2 < maturity: index(t1) <= index(t2)
   ```

4. **Par Convergence**:
   ```
   index(maturity_time) = SCALE (representing par)
   ```

5. **KYC Requirement**:
   ```
   subscribe() requires is_kyc_verified(user) = true
   redeem() requires is_kyc_verified(user) = true
   ```

### Attack Vectors & Mitigations

#### 1. Cap Bypass
**Attack**: Subscribe multiple times to exceed per_user_cap
**Mitigation**: Accumulate shares in single UserPosition

#### 2. Time Manipulation
**Attack**: Manipulate block timestamp
**Mitigation**: Soroban ledger timestamp is consensus-derived, minimal drift

#### 3. Front-running
**Attack**: MEV bots front-run subscriptions
**Impact**: Minimal (yield determined by time, not price discovery)

#### 4. Reentrancy
**Risk**: USDC transfer callbacks
**Mitigation**: Follow checks-effects-interactions pattern

#### 5. Integer Overflow
**Risk**: Large multiplications in yield calculation
**Mitigation**: Rust's overflow checks in release mode

## Gas Optimization

### Storage Access Patterns
- Use `persistent()` for long-lived data (Series, UserPosition)
- Use `instance()` for singleton data (Admin, NextSeriesId)
- Minimize storage writes

### Computation
- Index calculation is O(1) - no loops
- Single storage read for series data
- Single storage write for user position

## Testing Strategy

### Unit Tests
- [x] Series issuance validation
- [x] Index calculation at various times
- [x] Share calculation
- [x] Cap enforcement
- [x] KYC checks
- [x] Redemption logic

### Integration Tests
- [ ] Full lifecycle: issue → subscribe → redeem
- [ ] Multiple users
- [ ] Cap limit scenarios
- [ ] KYC revocation during active position

### Fuzzing Targets
- [ ] Index calculation for all time ranges
- [ ] Share calculation for all amounts
- [ ] Cap enforcement edge cases

## Deployment Checklist

### Pre-deployment
- [ ] Run full test suite
- [ ] Audit contract code
- [ ] Verify WASM build deterministic
- [ ] Test on testnet

### Deployment
- [ ] Deploy contract
- [ ] Initialize with multisig admin
- [ ] Verify contract on explorer
- [ ] Document contract ID

### Post-deployment
- [ ] Test initialize function
- [ ] Test KYC verification
- [ ] Test issue_series
- [ ] Monitor first subscriptions
- [ ] Set up event monitoring

## Future Enhancements

### Phase 2 (Post-MVP)
- [ ] Repo mechanics
- [ ] Secondary market transfers
- [ ] Batch operations
- [ ] Event emission for indexing
- [ ] Emergency pause

### Phase 3 (Advanced)
- [ ] Yield index snapshots
- [ ] Historical data queries
- [ ] Governance integration
- [ ] Cross-series operations

## Known Limitations

1. **No events**: Current version doesn't emit events (adds to README as TODO)
2. **No pause**: No emergency pause mechanism
3. **Single asset**: Only supports USDC
4. **No transfer**: Cannot transfer positions between users
5. **Manual settlement**: Admin must manually settle at maturity

## Performance Characteristics

- **Gas per subscribe**: ~50k (estimate)
- **Gas per redeem**: ~40k (estimate)
- **Max series**: Unlimited (constrained by NextSeriesId: u32)
- **Max users per series**: Unlimited
- **Storage per series**: ~256 bytes
- **Storage per user position**: ~64 bytes

## Maintenance

### Monitoring
Monitor:
- Series issuance events
- Subscription volumes
- Cap utilization
- Settlement timeliness
- KYC verification queue

### Upgrades
Contract is immutable by default. For upgrades:
1. Deploy new version
2. Migrate users to new contract
3. Deprecate old contract

Alternative: Implement upgradeable proxy pattern (future)
