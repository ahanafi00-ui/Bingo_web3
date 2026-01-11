#!/bin/bash

# BINGO Series Contract Deployment Script
# Usage: ./deploy.sh [network] [identity]
# Example: ./deploy.sh testnet alice

set -e

NETWORK=${1:-testnet}
IDENTITY=${2:-admin}

echo "🎯 BINGO Series Contract Deployment"
echo "=================================="
echo "Network: $NETWORK"
echo "Identity: $IDENTITY"
echo ""

# Build contract
echo "📦 Building contract..."
cd contracts/series
cargo build --target wasm32-unknown-unknown --release
cd ../..

# Optimize WASM (optional, requires wasm-opt)
if command -v wasm-opt &> /dev/null; then
    echo "⚡ Optimizing WASM..."
    wasm-opt -Oz \
        contracts/series/target/wasm32-unknown-unknown/release/bingo_series.wasm \
        -o contracts/series/target/wasm32-unknown-unknown/release/bingo_series_opt.wasm
    WASM_PATH="contracts/series/target/wasm32-unknown-unknown/release/bingo_series_opt.wasm"
else
    echo "⚠️  wasm-opt not found, skipping optimization"
    WASM_PATH="contracts/series/target/wasm32-unknown-unknown/release/bingo_series.wasm"
fi

# Deploy
echo "🚀 Deploying to $NETWORK..."
CONTRACT_ID=$(soroban contract deploy \
    --wasm $WASM_PATH \
    --source $IDENTITY \
    --network $NETWORK)

echo ""
echo "✅ Contract deployed successfully!"
echo "Contract ID: $CONTRACT_ID"
echo ""

# Save contract ID
mkdir -p deployments
echo $CONTRACT_ID > deployments/${NETWORK}_contract_id.txt

echo "📝 Contract ID saved to: deployments/${NETWORK}_contract_id.txt"
echo ""
echo "🔧 Next steps:"
echo "1. Initialize contract:"
echo "   soroban contract invoke \\"
echo "     --id $CONTRACT_ID \\"
echo "     --source $IDENTITY \\"
echo "     --network $NETWORK \\"
echo "     -- initialize \\"
echo "     --admin <ADMIN_ADDRESS>"
echo ""
echo "2. Verify KYC for users:"
echo "   soroban contract invoke \\"
echo "     --id $CONTRACT_ID \\"
echo "     --source $IDENTITY \\"
echo "     --network $NETWORK \\"
echo "     -- verify_kyc \\"
echo "     --user <USER_ADDRESS>"
echo ""
echo "3. Issue a series:"
echo "   soroban contract invoke \\"
echo "     --id $CONTRACT_ID \\"
echo "     --source $IDENTITY \\"
echo "     --network $NETWORK \\"
echo "     -- issue_series \\"
echo "     --par_value 1000000 \\"
echo "     --subscription_price 980000 \\"
echo "     --maturity_time <TIMESTAMP> \\"
echo "     --max_cap 10000000 \\"
echo "     --per_user_cap 1000000 \\"
echo "     --usdc_token <USDC_ADDRESS>"
