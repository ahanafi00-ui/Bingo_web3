#!/bin/bash
# Example: Create a T-Bill Series
# Usage: ./create_series.sh [deployment_file]

set -e

DEPLOYMENT_FILE=${1:-"deployment_testnet.json"}

if [ ! -f "$DEPLOYMENT_FILE" ]; then
    echo "❌ Deployment file not found: $DEPLOYMENT_FILE"
    exit 1
fi

# Load deployment info
VAULT_ID=$(jq -r '.contracts.bingo_vault' "$DEPLOYMENT_FILE")
RPC_URL=$(jq -r '.rpc_url' "$DEPLOYMENT_FILE")
NETWORK=$(jq -r '.network' "$DEPLOYMENT_FILE")

if [ "$NETWORK" = "testnet" ]; then
    NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
else
    NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
fi

if [ -z "$TREASURY_SECRET_KEY" ]; then
    echo "❌ TREASURY_SECRET_KEY environment variable not set"
    exit 1
fi

echo "📝 Creating T-Bill Series"
echo "=========================="
echo ""

# Example: 90-day T-Bill at 5% discount
SERIES_ID=1
ISSUE_DATE=$(date +%s)
MATURITY_DATE=$((ISSUE_DATE + 90 * 24 * 3600))  # 90 days from now
ISSUE_PRICE=9500000  # 0.95 (5% discount)
CAP_PAR=1000000000000000  # 100M in PAR units
USER_CAP_PAR=10000000000000  # 1M per user

echo "Series Parameters:"
echo "  Series ID: $SERIES_ID"
echo "  Issue Date: $(date -d @$ISSUE_DATE)"
echo "  Maturity Date: $(date -d @$MATURITY_DATE)"
echo "  Issue Price: 0.95 (5% discount)"
echo "  Series Cap: 100,000,000 PAR"
echo "  User Cap: 1,000,000 PAR"
echo ""

read -p "Create this series? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 1
fi

echo "Creating series..."
soroban contract invoke \
  --id "$VAULT_ID" \
  --source "$TREASURY_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- create_series \
  --series_id "$SERIES_ID" \
  --issue_date "$ISSUE_DATE" \
  --maturity_date "$MATURITY_DATE" \
  --issue_price "$ISSUE_PRICE" \
  --cap_par "$CAP_PAR" \
  --user_cap_par "$USER_CAP_PAR"

echo ""
echo "✅ Series $SERIES_ID created successfully!"
echo ""
echo "Next step: Activate the series"
echo "  ./activate_series.sh $SERIES_ID"
