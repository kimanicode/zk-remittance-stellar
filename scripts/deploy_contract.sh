#!/bin/bash
set -e

echo "=== ZK Remittance Contract Deployment ==="

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Build the contract
echo "[1/4] Building contract WASM..."
cd "$PROJECT_DIR/contracts"
cargo build --target wasm32v1-none --release
cd "$PROJECT_DIR"

WASM="$PROJECT_DIR/contracts/target/wasm32v1-none/release/zk_remittance.wasm"
if [ ! -f "$WASM" ]; then
    echo "ERROR: WASM not found at $WASM"
    exit 1
fi

# Upload contract
echo "[2/4] Uploading WASM to Stellar testnet..."
UPLOAD_OUTPUT=$(stellar contract upload \
    --network testnet \
    --source alice \
    --wasm "$WASM" 2>&1)
echo "$UPLOAD_OUTPUT"
WASM_HASH=$(echo "$UPLOAD_OUTPUT" | grep -oP '[0-9a-f]{64}' | tail -1)

# Deploy contract
echo "[3/4] Deploying contract instance..."
CONTRACT_ID=$(stellar contract deploy \
    --network testnet \
    --source alice \
    --wasm "$WASM" 2>&1 | grep -oP 'C[A-Z0-9]+')

echo "Contract deployed: $CONTRACT_ID"
echo "$CONTRACT_ID" > "$PROJECT_DIR/.contract_id"

# Initialize
echo "[4/4] Initializing contract..."
MERKLE_ROOT_HEX=$(cat "$PROJECT_DIR/circuits/build/merkle_root.hex" 2>/dev/null || echo "0000000000000000000000000000000000000000000000000000000000000000")

# For demo, use the USDC testnet token address
TESTNET_USDC="CAQAAONVIWQIYEGRMEI2PMT4K3WGI2GS77YY2R3BHWCO7VSPUQJBHM6H"

stellar contract invoke \
    --network testnet \
    --source alice \
    --id "$CONTRACT_ID" \
    -- initialize \
    --admin "$(stellar keys address alice)" \
    --merkle_root "$MERKLE_ROOT_HEX" \
    --token "$TESTNET_USDC"

echo ""
echo "=== Deployment complete ==="
echo "Contract ID: $CONTRACT_ID"
echo "Contract ID saved to .contract_id"
