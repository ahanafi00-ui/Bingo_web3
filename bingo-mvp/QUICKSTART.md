# Quick Start Guide

Get BINGO Series contract running in 5 minutes.

## Prerequisites

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 2. Add wasm target
rustup target add wasm32-unknown-unknown

# 3. Install Soroban CLI
cargo install --locked soroban-cli

# 4. Verify installation
soroban --version
```

## Setup

```bash
# Clone or navigate to project
cd bingo-mvp

# Configure Stellar testnet
soroban config network add testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"

# Generate admin identity
soroban keys generate admin --network testnet

# Get testnet XLM (for gas)
# Visit: https://laboratory.stellar.org/#account-creator?network=test
# Enter address: $(soroban keys address admin)
```

## Deploy

```bash
# Build and deploy
./deploy.sh testnet admin

# Output will show contract ID, save it:
# Contract ID: CA...ABC
```

## Initialize

```bash
# Set your contract ID
CONTRACT_ID="CA...ABC"  # Replace with your actual contract ID

# Initialize contract
soroban contract invoke \
  --id $CONTRACT_ID \
  --source admin \
  --network testnet \
  -- initialize \
  --admin $(soroban keys address admin)
```

## Create Test User

```bash
# Generate user identity
soroban keys generate alice --network testnet

# Verify KYC for Alice
soroban contract invoke \
  --id $CONTRACT_ID \
  --source admin \
  --network testnet \
  -- verify_kyc \
  --user $(soroban keys address alice)

# Check KYC status
soroban contract invoke \
  --id $CONTRACT_ID \
  --source admin \
  --network testnet \
  -- is_kyc_verified \
  --user $(soroban keys address alice)
```

## Issue Series

```bash
# Calculate maturity (90 days from now)
MATURITY=$(date -d "+90 days" +%s)

# Issue obligation series
# Note: Replace USDC_TOKEN with actual testnet USDC address
USDC_TOKEN="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"

SERIES_ID=$(soroban contract invoke \
  --id $CONTRACT_ID \
  --source admin \
  --network testnet \
  -- issue_series \
  --par_value 1000000 \
  --subscription_price 980000 \
  --maturity_time $MATURITY \
  --max_cap 10000000 \
  --per_user_cap 1000000 \
  --usdc_token $USDC_TOKEN)

echo "Series created: $SERIES_ID"
```

## View Series Info

```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --source admin \
  --network testnet \
  -- get_series \
  --series_id $SERIES_ID
```

## Subscribe (Requires USDC)

```bash
# First, get testnet USDC tokens
# Visit Stellar testnet asset issuer or use test faucet

# Then subscribe
soroban contract invoke \
  --id $CONTRACT_ID \
  --source alice \
  --network testnet \
  -- subscribe \
  --series_id $SERIES_ID \
  --usdc_amount 980000 \
  --user $(soroban keys address alice)
```

## Check Position

```bash
# Get user position
soroban contract invoke \
  --id $CONTRACT_ID \
  --source admin \
  --network testnet \
  -- get_user_position \
  --series_id $SERIES_ID \
  --user $(soroban keys address alice)

# Get current value (with accrued yield)
soroban contract invoke \
  --id $CONTRACT_ID \
  --source admin \
  --network testnet \
  -- get_position_value \
  --series_id $SERIES_ID \
  --user $(soroban keys address alice)
```

## After Maturity

```bash
# 1. Admin settles series
soroban contract invoke \
  --id $CONTRACT_ID \
  --source admin \
  --network testnet \
  -- settle_series \
  --series_id $SERIES_ID \
  --usdc_amount 1000000 \
  --admin $(soroban keys address admin)

# 2. User redeems
soroban contract invoke \
  --id $CONTRACT_ID \
  --source alice \
  --network testnet \
  -- redeem \
  --series_id $SERIES_ID \
  --user $(soroban keys address alice)
```

## Common Issues

### Issue: "Account not found"
**Solution**: Fund account with testnet XLM from faucet

### Issue: "Contract not found"
**Solution**: Verify CONTRACT_ID is correct, check deployment succeeded

### Issue: "Not KYC verified"
**Solution**: Admin must call `verify_kyc` for user first

### Issue: "Exceeds per-user cap"
**Solution**: Reduce subscription amount or increase per_user_cap when issuing series

## Next Steps

- Read [README.md](README.md) for detailed documentation
- Check [SPECIFICATION.md](SPECIFICATION.md) for technical details
- Run `./examples.sh` for more usage examples
- Explore contract on [Stellar Expert](https://stellar.expert/explorer/testnet)

## Helpful Commands

```bash
# List all identities
soroban keys list

# Get address for identity
soroban keys address <identity>

# Check contract on explorer
echo "https://stellar.expert/explorer/testnet/contract/$CONTRACT_ID"

# Run tests
cd contracts/series && cargo test

# Rebuild contract
cd contracts/series && \
  cargo build --target wasm32-unknown-unknown --release
```

## Support

- GitHub Issues: [Create issue](https://github.com/your-org/bingo-protocol/issues)
- Discord: [Join server](https://discord.gg/bingo)
- Docs: [Full documentation](https://docs.bingoprotocol.com)
