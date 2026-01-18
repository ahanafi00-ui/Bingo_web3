# BINGO Protocol - Refactored Structure

## 📁 New File Structure

```
contracts/
├── bt_bill_token/
│   ├── src/
│   │   ├── lib.rs          # Main contract implementation
│   │   ├── error.rs        # Error definitions
│   │   ├── storage.rs      # Storage types & keys
│   │   └── events.rs       # Event definitions
│   └── Cargo.toml
│
├── bingo_vault/
│   ├── src/
│   │   ├── lib.rs          # Main contract implementation
│   │   ├── error.rs        # Comprehensive error types
│   │   ├── storage.rs      # Series, UserPosition, DataKey
│   │   ├── events.rs       # All event types
│   │   └── pricing.rs      # Price calculation logic (NEW)
│   └── Cargo.toml
│
└── repo_market/
    ├── src/
    │   ├── lib.rs          # Main contract implementation
    │   ├── error.rs        # Repo-specific errors
    │   ├── storage.rs      # RepoPosition, DataKey
    │   ├── events.rs       # Repo events
    │   └── validation.rs   # Haircut & LTV validation (NEW)
    └── Cargo.toml
```

---

## 🔴 Error Organization

### **bt_bill_token/error.rs**
```rust
pub enum Error {
    // Initialization (1-2)
    AlreadyInitialized = 1,
    NotInitialized = 2,
    
    // Authorization (3, 6)
    Unauthorized = 3,
    NotOperator = 6,
    
    // Balance (4-5)
    InsufficientBalance = 4,
    InvalidAmount = 5,
}
```

### **bingo_vault/error.rs**
```rust
pub enum Error {
    // Initialization (1-5)
    AlreadyInitialized = 1,
    NotInitialized = 2,
    
    // Authorization (10-15)
    Unauthorized = 10,  // Not admin/treasury
    
    // Series Management (20-29)
    SeriesNotFound = 20,
    SeriesAlreadyExists = 21,
    SeriesNotActive = 22,      // ← User tries to subscribe before activation
    SeriesNotMatured = 23,     // ← User tries to redeem too early
    InvalidStatus = 24,
    
    // Subscription (30-39)
    ExceedsSeriesCap = 30,     // ← User hits series limit
    ExceedsUserCap = 31,       // ← User hits personal limit
    
    // Amount (40-49)
    InvalidAmount = 40,
    InsufficientBalance = 41,
    
    // Validation (50-59)
    InvalidTimestamp = 50,      // ← Maturity before issue
    InvalidIssuePrice = 51,     // ← Price not in (0, PAR]
    InvalidCapAmounts = 52,     // ← user_cap > series_cap
    
    // Operational (60-69)
    ContractPaused = 60,
}
```

### **repo_market/error.rs**
```rust
pub enum Error {
    // Initialization (1-5)
    AlreadyInitialized = 1,
    NotInitialized = 2,
    
    // Authorization (10-15)
    Unauthorized = 10,  // Not treasury
    
    // Position (20-29)
    PositionNotFound = 20,
    InvalidStatus = 21,
    
    // Amount (30-39)
    InvalidAmount = 30,
    ExceedsMaxCash = 31,        // ← Borrow more than LTV allows
    
    // Deadline (40-49)
    InvalidDeadline = 40,       // ← Deadline > maturity
    DeadlineNotPassed = 41,     // ← Try to claim before deadline
    DeadlinePassed = 42,        // ← Try to close after deadline
    
    // Operational (50-59)
    ContractPaused = 50,
}
```

---

## 📋 Error Messages by Flow

### **Flow 1: Treasury Creates Series**
```rust
// Success path
create_series(...)  // ✓

// Error scenarios
create_series(...) → Unauthorized (10)
  // Reason: Caller is not admin/treasury
  // Solution: Use treasury address

create_series(series_id: 1, ...) // first time ✓
create_series(series_id: 1, ...) // again → SeriesAlreadyExists (21)
  // Reason: Series ID already used
  // Solution: Use different series_id

create_series(
    issue_date: 2000,
    maturity_date: 1000  // ← ERROR
) → InvalidTimestamp (50)
  // Reason: Maturity must be after issue
  // Solution: Fix dates

create_series(
    issue_price: 1.5  // ← ERROR
) → InvalidIssuePrice (51)
  // Reason: Price must be ≤ PAR (1.0)
  // Solution: Use valid discount (e.g., 0.95)

create_series(
    cap_par: 100,
    user_cap_par: 200  // ← ERROR
) → InvalidCapAmounts (52)
  // Reason: user_cap cannot exceed series_cap
  // Solution: user_cap_par ≤ cap_par
```

### **Flow 2: Treasury Activates Series**
```rust
// Success path
activate_series(series_id: 1)  // ✓

// Error scenarios
activate_series(999) → SeriesNotFound (20)
  // Reason: Series doesn't exist
  // Solution: Create series first

activate_series(1) // when status != UPCOMING → InvalidStatus (24)
  // Reason: Can only activate UPCOMING series
  // Solution: Check series status
```

### **Flow 3: User Subscribes**
```rust
// Success path
subscribe(user: Alice, series_id: 1, pay_amount: 1000)  // ✓

// Error scenarios
subscribe(user: Alice, series_id: 999, ...) → SeriesNotFound (20)
  // Reason: Series doesn't exist
  // Solution: Use valid series_id

subscribe(user: Alice, series_id: 1, ...) // when status = UPCOMING
  → SeriesNotActive (22)
  // Reason: Series not yet activated
  // Solution: Wait for treasury to activate

subscribe(user: Alice, series_id: 1, pay_amount: 0) 
  → InvalidAmount (40)
  // Reason: Amount must be positive
  // Solution: pay_amount > 0

subscribe(user: Alice, pay_amount: 999999999) 
  → ExceedsSeriesCap (30)
  // Reason: Would exceed series cap_par
  // Solution: Reduce amount or wait for new series

subscribe(user: Alice, pay_amount: 100000) 
  → ExceedsUserCap (31)
  // Reason: Would exceed user's personal cap_par
  // Solution: Reduce amount (anti-monopoly protection)
```

### **Flow 4: User Redeems**
```rust
// Success path
redeem(user: Alice, series_id: 1, bt_bill_amount: 1000)  // ✓ (at maturity)

// Error scenarios
redeem(...) // before maturity → SeriesNotMatured (23)
  // Reason: Can only redeem at/after maturity_date
  // Solution: Wait until maturity

redeem(user: Alice, bt_bill_amount: 999999) 
  → InsufficientBalance (41)
  // Reason: User doesn't have enough bT-Bills
  // Solution: Check balance first
```

### **Flow 5: User Opens Repo**
```rust
// Success path
open_repo(
    borrower: Alice,
    series_id: 1,
    collateral_par: 10000,
    desired_cash_out: 9000,
    deadline: maturity - 30 days
)  // ✓

// Error scenarios
open_repo(collateral_par: 0, ...) → InvalidAmount (30)
  // Reason: Must provide collateral
  // Solution: collateral_par > 0

open_repo(
    collateral_par: 10000,
    desired_cash_out: 20000  // ← too much!
) → ExceedsMaxCash (31)
  // Reason: Requested more than LTV allows
  // Formula: max_cash = collateral × price × (1 - haircut)
  // Solution: Reduce desired_cash_out

open_repo(
    deadline: maturity_date + 10 days  // ← ERROR
) → InvalidDeadline (40)
  // Reason: Deadline must be ≤ maturity
  // Solution: deadline ≤ maturity_date
```

### **Flow 6: User Closes Repo (Repay)**
```rust
// Success path
close_repo(position_id: 1)  // ✓ (before deadline)

// Error scenarios
close_repo(999) → PositionNotFound (20)
  // Reason: Position doesn't exist
  // Solution: Use valid position_id

close_repo(1) // after deadline → DeadlinePassed (42)
  // Reason: Deadline already passed, defaulted
  // Solution: Position already lost to lender
```

### **Flow 7: Treasury Claims Default**
```rust
// Success path
claim_default(position_id: 1)  // ✓ (after deadline)

// Error scenarios
claim_default(1) // before deadline → DeadlineNotPassed (41)
  // Reason: Borrower still has time to repay
  // Solution: Wait until after deadline

claim_default(1) // not treasury → Unauthorized (10)
  // Reason: Only treasury can claim
  // Solution: Use treasury address
```

---

## 🎯 Error Handling Best Practices

### **1. Always Check Status First**
```rust
// ❌ Bad
fn subscribe(...) {
    let series = get_series()?;  // might not exist
    // ... proceed
}

// ✅ Good
fn subscribe(...) {
    let series = get_series().map_err(|_| Error::SeriesNotFound)?;
    
    if series.status != SeriesStatus::Active {
        return Err(Error::SeriesNotActive);
    }
    
    // ... proceed
}
```

### **2. Validate Inputs Early**
```rust
// ✅ Check at function start
pub fn subscribe(env: Env, user: Address, series_id: u32, pay_amount: i128) {
    Self::check_not_paused(&env)?;  // First
    
    if pay_amount <= 0 {
        return Err(Error::InvalidAmount);  // Early return
    }
    
    user.require_auth();
    
    // ... proceed with business logic
}
```

### **3. Provide Context in Errors**
```rust
// Error messages should be self-explanatory:
ExceedsSeriesCap  // ← Clear: "You hit the series limit"
ExceedsUserCap    // ← Clear: "You hit your personal limit"
InvalidAmount     // ← Clear: "Amount must be positive"
```

---

## 📖 Usage Examples

### **Frontend Error Handling**
```typescript
try {
    await vault.subscribe(userId, seriesId, amount);
} catch (error) {
    switch(error.code) {
        case 20: // SeriesNotFound
            showError("Series does not exist");
            break;
        case 22: // SeriesNotActive
            showError("Series not yet active. Wait for treasury to activate.");
            break;
        case 30: // ExceedsSeriesCap
            showError("Series is full. Try a smaller amount or wait for new series.");
            break;
        case 31: // ExceedsUserCap
            showError(`You can only buy up to ${userCap} PAR in this series (anti-monopoly rule)`);
            break;
        case 40: // InvalidAmount
            showError("Amount must be greater than zero");
            break;
        default:
            showError("Transaction failed");
    }
}
```

### **CLI Error Handling**
```bash
$ soroban contract invoke --id $VAULT -- subscribe \
    --user $USER \
    --series_id 1 \
    --pay_amount 1000000000

Error: ExceedsUserCap (31)
Reason: Subscription would exceed your personal cap of 100,000 PAR
Current: 95,000 PAR
Requested: 10,000 PAR (would be 105,000 total)
Solution: Try amount ≤ 5,000 PAR
```

---

## 🎓 Key Takeaways

1. **Errors are numbered by category** (10s, 20s, 30s, etc.) for easy organization
2. **Each error has a specific meaning** tied to business logic
3. **Errors guide users** to the solution (e.g., "reduce amount" vs "wait for activation")
4. **Authorization errors** (Unauthorized) tell you "who can do this action"
5. **Validation errors** (InvalidAmount, ExceedsCap) tell you "what's wrong with your input"
6. **State errors** (NotActive, NotMatured) tell you "when you can do this action"

This refactored structure makes it **crystal clear** why a transaction failed and how to fix it! 🎯
