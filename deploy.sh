#!/bin/bash
# BINGO Protocol Deployment Script
# Usage: ./deploy.sh [testnet|mainnet]

set -e

NETWORK=${1:-testnet}

if [ "$NETWORK" = "testnet" ]; then
    RPC_URL="https://soroban-testnet.stellar.org"
    NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
elif [ "$NETWORK" = "mainnet" ]; then
    RPC_URL="https://soroban-mainnet.stellar.org"
    NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
else
    echo "Invalid network. Use 'testnet' or 'mainnet'"
    exit 1
fi

echo "🚀 Deploying BINGO Protocol to $NETWORK"
echo "================================================"

# Check if contracts are built
if [ ! -f "target/wasm32-unknown-unknown/release/bt_bill_token.wasm" ]; then
    echo "❌ Contracts not built. Run 'cargo build --target wasm32-unknown-unknown --release' first"
    exit 1
fi

# Load admin secret key from environment or file
if [ -z "$ADMIN_SECRET_KEY" ]; then
    echo "❌ ADMIN_SECRET_KEY environment variable not set"
    exit 1
fi

if [ -z "$TREASURY_ADDRESS" ]; then
    echo "❌ TREASURY_ADDRESS environment variable not set"
    exit 1
fi

if [ -z "$STABLECOIN_ID" ]; then
    echo "❌ STABLECOIN_ID environment variable not set (USDC contract address)"
    exit 1
fi

echo "📝 Configuration:"
echo "  Network: $NETWORK"
echo "  RPC: $RPC_URL"
echo "  Treasury: $TREASURY_ADDRESS"
echo "  Stablecoin: $STABLECOIN_ID"
echo ""

# Deploy bt_bill_token
echo "1️⃣  Deploying bt_bill_token..."
BT_BILL_TOKEN_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/bt_bill_token.wasm \
  --source "$ADMIN_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  2>&1 | tail -n 1)

echo "   ✅ bt_bill_token deployed: $BT_BILL_TOKEN_ID"

# Initialize bt_bill_token
echo "2️⃣  Initializing bt_bill_token..."
ADMIN_ADDRESS=$(soroban keys address admin 2>/dev/null || echo "ADMIN_ADDRESS_HERE")
soroban contract invoke \
  --id "$BT_BILL_TOKEN_ID" \
  --source "$ADMIN_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- initialize \
  --admin "$ADMIN_ADDRESS" > /dev/null

echo "   ✅ bt_bill_token initialized"

# Deploy bingo_vault
echo "3️⃣  Deploying bingo_vault..."
VAULT_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/bingo_vault.wasm \
  --source "$ADMIN_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  2>&1 | tail -n 1)

echo "   ✅ bingo_vault deployed: $VAULT_ID"

# Initialize bingo_vault
echo "4️⃣  Initializing bingo_vault..."
soroban contract invoke \
  --id "$VAULT_ID" \
  --source "$ADMIN_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- initialize \
  --admin "$ADMIN_ADDRESS" \
  --treasury "$TREASURY_ADDRESS" \
  --stablecoin "$STABLECOIN_ID" \
  --bt_bill_token "$BT_BILL_TOKEN_ID" > /dev/null

echo "   ✅ bingo_vault initialized"

# Add vault as operator
echo "5️⃣  Adding vault as operator to bt_bill_token..."
soroban contract invoke \
  --id "$BT_BILL_TOKEN_ID" \
  --source "$ADMIN_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- add_operator \
  --operator "$VAULT_ID" > /dev/null

echo "   ✅ Vault added as operator"

# Deploy repo_market
echo "6️⃣  Deploying repo_market..."
REPO_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/repo_market.wasm \
  --source "$ADMIN_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  2>&1 | tail -n 1)

echo "   ✅ repo_market deployed: $REPO_ID"

# Initialize repo_market
echo "7️⃣  Initializing repo_market..."
soroban contract invoke \
  --id "$REPO_ID" \
  --source "$ADMIN_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- initialize \
  --admin "$ADMIN_ADDRESS" \
  --treasury "$TREASURY_ADDRESS" \
  --vault "$VAULT_ID" \
  --stablecoin "$STABLECOIN_ID" \
  --bt_bill_token "$BT_BILL_TOKEN_ID" \
  --haircut_bps 300 \
  --spread_bps 200 > /dev/null

echo "   ✅ repo_market initialized"

# Add repo as operator
echo "8️⃣  Adding repo as operator to bt_bill_token..."
soroban contract invoke \
  --id "$BT_BILL_TOKEN_ID" \
  --source "$ADMIN_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- add_operator \
  --operator "$REPO_ID" > /dev/null

echo "   ✅ Repo added as operator"

# Save deployment addresses
DEPLOYMENT_FILE="deployment_${NETWORK}.json"
cat > "$DEPLOYMENT_FILE" <<EOF
{
  "network": "$NETWORK",
  "rpc_url": "$RPC_URL",
  "contracts": {
    "bt_bill_token": "$BT_BILL_TOKEN_ID",
    "bingo_vault": "$VAULT_ID",
    "repo_market": "$REPO_ID"
  },
  "addresses": {
    "admin": "$ADMIN_ADDRESS",
    "treasury": "$TREASURY_ADDRESS",
    "stablecoin": "$STABLECOIN_ID"
  },
  "deployed_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

echo ""
echo "================================================"
echo "🎉 Deployment Complete!"
echo "================================================"
echo ""
echo "📄 Deployment details saved to: $DEPLOYMENT_FILE"
echo ""
echo "Contract Addresses:"
echo "  bt_bill_token: $BT_BILL_TOKEN_ID"
echo "  bingo_vault:   $VAULT_ID"
echo "  repo_market:   $REPO_ID"
echo ""
echo "Next steps:"
echo "  1. Create a T-Bill series: ./scripts/create_series.sh"
echo "  2. Activate the series: ./scripts/activate_series.sh"
echo "  3. Users can subscribe: ./scripts/subscribe.sh"
echo ""
