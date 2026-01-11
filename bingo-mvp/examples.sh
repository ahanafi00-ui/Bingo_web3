#!/bin/bash

# BINGO Series Contract - Example Usage
# This script shows common operations

set -e

NETWORK=${1:-testnet}
IDENTITY=${2:-admin}
CONTRACT_ID=$(cat deployments/${NETWORK}_contract_id.txt 2>/dev/null || echo "")

if [ -z "$CONTRACT_ID" ]; then
    echo "❌ Contract not deployed. Run ./deploy.sh first"
    exit 1
fi

echo "🎯 BINGO Series - Example Usage"
echo "==============================="
echo "Contract: $CONTRACT_ID"
echo "Network: $NETWORK"
echo ""

# Helper function to invoke contract
invoke() {
    soroban contract invoke \
        --id $CONTRACT_ID \
        --source $IDENTITY \
        --network $NETWORK \
        -- "$@"
}

# Example 1: Initialize (if not already done)
echo "1️⃣  Initializing contract..."
ADMIN_ADDRESS=$(soroban keys address $IDENTITY)
invoke initialize --admin $ADMIN_ADDRESS || echo "   Already initialized"
echo ""

# Example 2: Create test user and verify KYC
echo "2️⃣  Setting up test user..."
USER_IDENTITY="test_user"
soroban keys generate $USER_IDENTITY --network $NETWORK 2>/dev/null || true
USER_ADDRESS=$(soroban keys address $USER_IDENTITY)
echo "   User address: $USER_ADDRESS"

echo "   Verifying KYC..."
invoke verify_kyc --user $USER_ADDRESS
echo "   ✅ KYC verified"
echo ""

# Example 3: Issue a series (90-day T-Bill)
echo "3️⃣  Issuing obligation series..."
CURRENT_TIME=$(date +%s)
MATURITY_TIME=$((CURRENT_TIME + 7776000)) # 90 days

# Note: Replace with actual USDC token address
USDC_TOKEN="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"

SERIES_ID=$(invoke issue_series \
    --par_value 1000000 \
    --subscription_price 980000 \
    --maturity_time $MATURITY_TIME \
    --max_cap 10000000 \
    --per_user_cap 1000000 \
    --usdc_token $USDC_TOKEN)

echo "   Series ID: $SERIES_ID"
echo "   Par: 1,000,000 ($1.00)"
echo "   Subscription: 980,000 ($0.98)"
echo "   Yield: 2%"
echo "   Maturity: $(date -d @$MATURITY_TIME '+%Y-%m-%d')"
echo ""

# Example 4: Get series info
echo "4️⃣  Getting series information..."
SERIES_INFO=$(invoke get_series --series_id $SERIES_ID)
echo "$SERIES_INFO"
echo ""

# Example 5: Check user KYC status
echo "5️⃣  Checking KYC status..."
KYC_STATUS=$(invoke is_kyc_verified --user $USER_ADDRESS)
echo "   User $USER_ADDRESS KYC: $KYC_STATUS"
echo ""

# Example 6: Subscribe (requires USDC tokens)
echo "6️⃣  Subscribing to series..."
echo "   ⚠️  Note: This requires actual USDC tokens"
echo "   Command:"
echo "   invoke subscribe \\"
echo "     --series_id $SERIES_ID \\"
echo "     --usdc_amount 980000 \\"
echo "     --user $USER_ADDRESS"
echo ""

# Example 7: Get position value
echo "7️⃣  Getting position value..."
echo "   Command:"
echo "   invoke get_position_value \\"
echo "     --series_id $SERIES_ID \\"
echo "     --user $USER_ADDRESS"
echo ""

# Example 8: Settlement at maturity
echo "8️⃣  Settlement at maturity..."
echo "   After maturity timestamp, admin runs:"
echo "   invoke settle_series \\"
echo "     --series_id $SERIES_ID \\"
echo "     --usdc_amount 1000000 \\"
echo "     --admin $ADMIN_ADDRESS"
echo ""

# Example 9: Redemption
echo "9️⃣  User redemption..."
echo "   After settlement, user can redeem:"
echo "   invoke redeem \\"
echo "     --series_id $SERIES_ID \\"
echo "     --user $USER_ADDRESS"
echo ""

echo "✅ Example operations complete!"
echo ""
echo "📚 For more info, see README.md"
