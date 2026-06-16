#!/bin/bash
set -e

echo "=== ZK Remittance Circuit Compilation ==="

cd "$(dirname "$0")/../circuits"

# Install circomlib if not present
if [ ! -d "node_modules/circomlib" ]; then
    echo "[1/6] Installing circomlib..."
    npm init -y > /dev/null 2>&1
    npm install circomlib
fi

echo "[2/6] Compiling circuit with circom..."
circom remittance.circom --r1cs --wasm --sym -o build/

echo "[3/6] Setting up Groth16 with Powers of Tau..."
cd build

POT_FILE="pot17_final.ptau"
if [ ! -f "$POT_FILE" ]; then
    echo "     Downloading Powers of Tau from Hermez..."
    wget -q https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_17.ptau -O "$POT_FILE"
fi

echo "[4/6] Generating proving key..."
snarkjs groth16 setup remittance.r1cs "$POT_FILE" remittance_0000.zkey

echo "[5/6] Contributing to ceremony..."
snarkjs zkey contribute remittance_0000.zkey remittance_final.zkey --name="zk-remittance" -v

echo "[6/6] Exporting verification key..."
snarkjs zkey export verificationkey remittance_final.zkey verification_key.json

echo ""
echo "=== Compilation complete ==="
echo "Files in build/:"
ls -lh remittance.r1cs remittance.wasm remittance_final.zkey verification_key.json
