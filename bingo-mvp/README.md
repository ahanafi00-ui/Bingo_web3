# BINGO Series Contract MVP (Refactored)

Smart contract untuk BINGO Protocol - Onchain T-Bill obligation primitive on Stellar Soroban.

## 🏗️ Project Structure

```
contracts/series/src/
├── lib.rs          # Main contract interface
├── types.rs        # Data structures & constants
├── storage.rs      # Storage access layer
├── admin.rs        # Admin functions (issue, KYC, settlement)
├── user_ops.rs     # User operations (subscribe, redeem)
├── validation.rs   # Business logic validation
└── yield_calc.rs   # Yield calculation engine
```

### Module Responsibilities

#### `types.rs`
- Data structures: `Series`, `SeriesStatus`, `UserPosition`
- Storage keys: `DataKey`
- Constants: `SCALE = 1e7`

#### `storage.rs`
- Storage abstraction layer
- CRUD operations for Series, UserPosition, KYC
- Admin management

#### `validation.rs`
- Business rule validation
- Series parameter checks
- Subscription constraints
- Redemption requirements

#### `yield_calc.rs`
- Index calculation (time-weighted)
- Share calculation
- Position value calculation
- Redemption value calculation
- **Includes unit tests**

#### `admin.rs`
- `initialize()`: Set admin
- `issue_series()`: Create new obligation
- `verify_kyc()` / `revoke_kyc()`: KYC management
- `settle_series()`: Deposit funds at maturity

#### `user_ops.rs`
- `subscribe()`: Buy obligation with cap checks
- `redeem()`: Claim at maturity
- `get_position_value()`: View current value

#### `lib.rs`
- Contract interface (Soroban SDK)
- Function routing
- Integration tests

## ✅ Features

### Core Functionality
- **Issue Obligation Series**: Admin bisa create series dengan maturity, caps, dan pricing
- **Subscribe dengan Cap Limits**: User subscribe dengan max cap (total & per-user)
- **Yield Calculation**: Automatic time-weighted yield accrual via index
- **KYC/AML**: Built-in verification requirement
- **Redemption**: Redeem at par value saat maturity

### Code Quality
- ✅ Modular architecture (7 modules)
- ✅ Separation of concerns
- ✅ Unit tests per module
- ✅ Integration tests
- ✅ Clear function responsibilities

## 🚀 Quick Start

```bash
# Build
cd contracts/series
cargo build --target wasm32-unknown-unknown --release

# Test
cargo test

# Deploy
./deploy.sh testnet admin
```

Full setup guide: [QUICKSTART.md](QUICKSTART.md)

## 📖 Documentation

- [README.md](README.md) - This file
- [SPECIFICATION.md](SPECIFICATION.md) - Technical specs
- [QUICKSTART.md](QUICKSTART.md) - 5-minute setup

## License

MIT
