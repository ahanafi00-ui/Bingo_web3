# BINGO Protocol - Build Instructions

## Fixed Issues

The contracts have been updated to fix Soroban SDK API compatibility issues:

1. **Auth Pattern**: Changed from `env.auths()` to explicit `user.require_auth()` parameters
2. **Function Signatures**: Added explicit `Address` parameters for user operations
3. **invoke_contract**: Updated to use `soroban_sdk::vec!` macro for arguments

## Updated Function Signatures

### bingo_vault
- `subscribe(env, user: Address, series_id, pay_amount)` - user parameter added
- `redeem(env, user: Address, series_id, bt_bill_amount)` - user parameter added

### repo_market  
- `open_repo(env, borrower: Address, series_id, collateral_par, desired_cash_out, deadline)` - borrower parameter added

## Building

```bash
# Build all contracts
cargo build --target wasm32-unknown-unknown --release

# Check for any remaining warnings
cargo clippy --target wasm32-unknown-unknown --release
```

## Testing

Note: Integration tests use `contractimport!` which requires compiled WASM files first.

```bash
# Build contracts first
cargo build --target wasm32-unknown-unknown --release

# Then run tests
cargo test --test integration_test
```

## Quick Start

1. **Build contracts:**
```bash
cargo build --target wasm32-unknown-unknown --release
```

2. **Deploy** (after build succeeds):
```bash
export ADMIN_SECRET_KEY="your_admin_key"
export TREASURY_ADDRESS="your_treasury_address"  
export STABLECOIN_ID="usdc_contract_id"

./deploy.sh testnet
```

3. **Create Series:**
```bash
export TREASURY_SECRET_KEY="your_treasury_key"
./scripts/create_series.sh
```

4. **User Subscribe:**
```bash
export USER_SECRET_KEY="user_key"
# Note: Soroban CLI will handle auth automatically
soroban contract invoke \
  --id $VAULT_ID \
  --source $USER_SECRET_KEY \
  -- subscribe \
  --user $USER_ADDRESS \
  --series_id 1 \
  --pay_amount 10000000000
```

## API Changes Summary

### Old (SDK 22 early versions):
```rust
pub fn subscribe(env: Env, series_id: u32, pay_amount: i128)
// Auth extracted from env.auths()
```

### New (SDK 22 current):
```rust
pub fn subscribe(env: Env, user: Address, series_id: u32, pay_amount: i128)  
// User explicitly passed, calls user.require_auth()
```

This pattern is cleaner and more explicit about who needs to authorize.

## Contract Invocations

Cross-contract calls now use:
```rust
env.invoke_contract(
    &contract_id,
    &Symbol::new(&env, "function_name"),
    soroban_sdk::vec![&env, arg1.into_val(&env), arg2.into_val(&env)]
)
```

## Deployment Order

1. Deploy bt_bill_token → Initialize → Note address
2. Deploy bingo_vault → Initialize with token address → Note address  
3. Add vault as operator to token
4. Deploy repo_market → Initialize with vault + token addresses → Note address
5. Add repo as operator to token
6. Ready to create series!

## Common Issues

**Issue**: `method 'auths' not found for Env`  
**Fix**: ✅ Fixed - now use explicit Address parameters

**Issue**: `trait bound Vec<Val>: From<(...)> not satisfied`  
**Fix**: ✅ Fixed - now use `soroban_sdk::vec!` macro

**Issue**: Integration tests fail  
**Cause**: Tests need compiled WASM files  
**Fix**: Build contracts first, then run tests

## Next Steps

After successful build:
- Review generated WASM files in `target/wasm32-unknown-unknown/release/`
- Test on Stellar testnet
- Run security audit before mainnet
- Update integration tests if needed

## Support

If you encounter build issues:
1. Check Rust version: `rustc --version` (need 1.70+)
2. Check target: `rustup target list | grep wasm32`
3. Update dependencies: `cargo update`
4. Clean build: `cargo clean && cargo build --target wasm32-unknown-unknown --release`
