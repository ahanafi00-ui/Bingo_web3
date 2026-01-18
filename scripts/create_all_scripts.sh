#!/bin/bash
# Collection of example scripts for BINGO protocol operations

# =============================================================================
# activate_series.sh - Activate a series
# =============================================================================
cat > scripts/activate_series.sh <<'ACTIVATE_EOF'
#!/bin/bash
set -e

SERIES_ID=${1:-1}
DEPLOYMENT_FILE=${2:-"deployment_testnet.json"}

VAULT_ID=$(jq -r '.contracts.bingo_vault' "$DEPLOYMENT_FILE")
RPC_URL=$(jq -r '.rpc_url' "$DEPLOYMENT_FILE")
NETWORK=$(jq -r '.network' "$DEPLOYMENT_FILE")

if [ "$NETWORK" = "testnet" ]; then
    NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
else
    NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
fi

echo "Activating Series $SERIES_ID..."

soroban contract invoke \
  --id "$VAULT_ID" \
  --source "$TREASURY_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- activate_series \
  --series_id "$SERIES_ID"

echo "✅ Series $SERIES_ID activated!"
ACTIVATE_EOF

chmod +x scripts/activate_series.sh

# =============================================================================
# subscribe.sh - User subscribes to a series
# =============================================================================
cat > scripts/subscribe.sh <<'SUBSCRIBE_EOF'
#!/bin/bash
set -e

SERIES_ID=${1:-1}
AMOUNT=${2:-10000000000}  # 1000 USDC default
DEPLOYMENT_FILE=${3:-"deployment_testnet.json"}

VAULT_ID=$(jq -r '.contracts.bingo_vault' "$DEPLOYMENT_FILE")
STABLECOIN_ID=$(jq -r '.addresses.stablecoin' "$DEPLOYMENT_FILE")
RPC_URL=$(jq -r '.rpc_url' "$DEPLOYMENT_FILE")
NETWORK=$(jq -r '.network' "$DEPLOYMENT_FILE")

if [ "$NETWORK" = "testnet" ]; then
    NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
else
    NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
fi

if [ -z "$USER_SECRET_KEY" ]; then
    echo "❌ USER_SECRET_KEY environment variable not set"
    exit 1
fi

echo "Subscribing to Series $SERIES_ID with $AMOUNT units..."
echo ""

# Note: User needs to have approved the vault to spend their stablecoin
# This would be done via the stablecoin contract's approve function

soroban contract invoke \
  --id "$VAULT_ID" \
  --source "$USER_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- subscribe \
  --series_id "$SERIES_ID" \
  --pay_amount "$AMOUNT"

echo "✅ Subscribed successfully!"
SUBSCRIBE_EOF

chmod +x scripts/subscribe.sh

# =============================================================================
# check_balance.sh - Check user's bT-Bill balance
# =============================================================================
cat > scripts/check_balance.sh <<'BALANCE_EOF'
#!/bin/bash
set -e

SERIES_ID=${1:-1}
USER_ADDRESS=${2}
DEPLOYMENT_FILE=${3:-"deployment_testnet.json"}

if [ -z "$USER_ADDRESS" ]; then
    echo "❌ Usage: ./check_balance.sh <series_id> <user_address>"
    exit 1
fi

BT_BILL_TOKEN_ID=$(jq -r '.contracts.bt_bill_token' "$DEPLOYMENT_FILE")
RPC_URL=$(jq -r '.rpc_url' "$DEPLOYMENT_FILE")
NETWORK=$(jq -r '.network' "$DEPLOYMENT_FILE")

if [ "$NETWORK" = "testnet" ]; then
    NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
else
    NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
fi

echo "Checking balance for Series $SERIES_ID, User: $USER_ADDRESS"
echo ""

BALANCE=$(soroban contract invoke \
  --id "$BT_BILL_TOKEN_ID" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- balance_of \
  --series_id "$SERIES_ID" \
  --user "$USER_ADDRESS")

# Convert from scaled units to human-readable
HUMAN_BALANCE=$(echo "scale=7; $BALANCE / 10000000" | bc)

echo "Balance: $HUMAN_BALANCE PAR units"
echo "  (raw: $BALANCE)"
BALANCE_EOF

chmod +x scripts/check_balance.sh

# =============================================================================
# get_price.sh - Get current price for a series
# =============================================================================
cat > scripts/get_price.sh <<'PRICE_EOF'
#!/bin/bash
set -e

SERIES_ID=${1:-1}
DEPLOYMENT_FILE=${2:-"deployment_testnet.json"}

VAULT_ID=$(jq -r '.contracts.bingo_vault' "$DEPLOYMENT_FILE")
RPC_URL=$(jq -r '.rpc_url' "$DEPLOYMENT_FILE")
NETWORK=$(jq -r '.network' "$DEPLOYMENT_FILE")

if [ "$NETWORK" = "testnet" ]; then
    NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
else
    NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
fi

echo "Getting current price for Series $SERIES_ID..."
echo ""

PRICE=$(soroban contract invoke \
  --id "$VAULT_ID" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- current_price \
  --series_id "$SERIES_ID")

HUMAN_PRICE=$(echo "scale=7; $PRICE / 10000000" | bc)

echo "Current Price: $HUMAN_PRICE"
echo "  (raw: $PRICE)"
PRICE_EOF

chmod +x scripts/get_price.sh

# =============================================================================
# redeem.sh - Redeem bT-Bills at maturity
# =============================================================================
cat > scripts/redeem.sh <<'REDEEM_EOF'
#!/bin/bash
set -e

SERIES_ID=${1:-1}
AMOUNT=${2}
DEPLOYMENT_FILE=${3:-"deployment_testnet.json"}

if [ -z "$AMOUNT" ]; then
    echo "❌ Usage: ./redeem.sh <series_id> <amount>"
    exit 1
fi

VAULT_ID=$(jq -r '.contracts.bingo_vault' "$DEPLOYMENT_FILE")
RPC_URL=$(jq -r '.rpc_url' "$DEPLOYMENT_FILE")
NETWORK=$(jq -r '.network' "$DEPLOYMENT_FILE")

if [ "$NETWORK" = "testnet" ]; then
    NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
else
    NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
fi

if [ -z "$USER_SECRET_KEY" ]; then
    echo "❌ USER_SECRET_KEY environment variable not set"
    exit 1
fi

echo "Redeeming $AMOUNT bT-Bills from Series $SERIES_ID..."
echo ""

soroban contract invoke \
  --id "$VAULT_ID" \
  --source "$USER_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- redeem \
  --series_id "$SERIES_ID" \
  --bt_bill_amount "$AMOUNT"

echo "✅ Redeemed successfully!"
REDEEM_EOF

chmod +x scripts/redeem.sh

# =============================================================================
# open_repo.sh - Open a repo position
# =============================================================================
cat > scripts/open_repo.sh <<'REPO_EOF'
#!/bin/bash
set -e

SERIES_ID=${1:-1}
COLLATERAL=${2}
CASH_REQUEST=${3}
DEADLINE=${4}
DEPLOYMENT_FILE=${5:-"deployment_testnet.json"}

if [ -z "$COLLATERAL" ] || [ -z "$CASH_REQUEST" ] || [ -z "$DEADLINE" ]; then
    echo "❌ Usage: ./open_repo.sh <series_id> <collateral_par> <cash_request> <deadline_timestamp>"
    exit 1
fi

REPO_ID=$(jq -r '.contracts.repo_market' "$DEPLOYMENT_FILE")
RPC_URL=$(jq -r '.rpc_url' "$DEPLOYMENT_FILE")
NETWORK=$(jq -r '.network' "$DEPLOYMENT_FILE")

if [ "$NETWORK" = "testnet" ]; then
    NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
else
    NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
fi

if [ -z "$USER_SECRET_KEY" ]; then
    echo "❌ USER_SECRET_KEY environment variable not set"
    exit 1
fi

echo "Opening repo position..."
echo "  Series: $SERIES_ID"
echo "  Collateral: $COLLATERAL PAR"
echo "  Cash Request: $CASH_REQUEST"
echo "  Deadline: $(date -d @$DEADLINE)"
echo ""

POSITION_ID=$(soroban contract invoke \
  --id "$REPO_ID" \
  --source "$USER_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- open_repo \
  --series_id "$SERIES_ID" \
  --collateral_par "$COLLATERAL" \
  --desired_cash_out "$CASH_REQUEST" \
  --deadline "$DEADLINE")

echo "✅ Repo position opened!"
echo "Position ID: $POSITION_ID"
REPO_EOF

chmod +x scripts/open_repo.sh

# =============================================================================
# close_repo.sh - Close a repo position
# =============================================================================
cat > scripts/close_repo.sh <<'CLOSE_EOF'
#!/bin/bash
set -e

POSITION_ID=${1}
DEPLOYMENT_FILE=${2:-"deployment_testnet.json"}

if [ -z "$POSITION_ID" ]; then
    echo "❌ Usage: ./close_repo.sh <position_id>"
    exit 1
fi

REPO_ID=$(jq -r '.contracts.repo_market' "$DEPLOYMENT_FILE")
RPC_URL=$(jq -r '.rpc_url' "$DEPLOYMENT_FILE")
NETWORK=$(jq -r '.network' "$DEPLOYMENT_FILE")

if [ "$NETWORK" = "testnet" ]; then
    NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
else
    NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
fi

if [ -z "$USER_SECRET_KEY" ]; then
    echo "❌ USER_SECRET_KEY environment variable not set"
    exit 1
fi

echo "Closing repo position $POSITION_ID..."

soroban contract invoke \
  --id "$REPO_ID" \
  --source "$USER_SECRET_KEY" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- close_repo \
  --position_id "$POSITION_ID"

echo "✅ Repo position closed!"
CLOSE_EOF

chmod +x scripts/close_repo.sh

echo "✅ All example scripts created in scripts/ directory"
