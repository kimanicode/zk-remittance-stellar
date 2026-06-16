# ZK Remittance on Stellar

Private cross-border remittance powered by zero-knowledge proofs on Stellar's Soroban platform. Send money under $10,000 with full KYC compliance — without revealing your identity, amount, or recipient to the public ledger.

### Why ZK

Without ZK, you face an impossible choice: full transparency (all transactions visible on-chain — no privacy) or full anonymity (no compliance — money laundering risk). ZK threads the needle by letting you prove "I am KYC-verified, my amount is under $10,000, and I haven't sent before" without revealing *which* KYC'd user you are, *how much* you're sending, or *who* the recipient is. The smart contract verifies one Groth16 proof and releases the funds — it learns only the Merkle root, a nullifier hash, and a recipient hash. Nothing else.

### Architecture

```
┌────────────────────────────────────────────────────────────┐
│                      Sender's Browser                       │
│  ┌──────────┐   ┌───────────┐   ┌──────────────────────┐  │
│  │ Circom   │   │  snarkjs  │   │  @stellar/stellar-sdk │  │
│  │ Circuit  │──>│ fullProve │──>│  submitTransaction    │  │
│  │ (WASM)   │   │ (Groth16) │   │                      │  │
│  └──────────┘   └───────────┘   └──────────┬─────────────┘  │
└─────────────────────────────────────────────┼────────────────┘
                                              │
                                              ▼
┌────────────────────────────────────────────────────────────┐
│                     Stellar Testnet                         │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Soroban Smart Contract                   │  │
│  │  ┌──────────┐   ┌──────────┐   ┌────────────────┐   │  │
│  │  │ Groth16  │──>│ Nullifier│──>│ Token Transfer │   │  │
│  │  │ Verify   │   │ Check    │   │ (USDC/XLM)     │   │  │
│  │  └──────────┘   └──────────┘   └────────────────┘   │  │
│  │                                                      │  │
│  │  ┌──────────────────────────────────────────────┐   │  │
│  │  │ Compliance Query (encrypted volume tally)    │   │  │
│  │  └──────────────────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────┘
```

### How to Run Locally

**Prerequisites:**
- Node.js 18+
- Rust 1.84+ with `wasm32v1-none` target (`rustup target add wasm32v1-none`)
- Circom 2.0+ (`npm install -g circom`)
- Stellar CLI (`cargo install stellar-cli --features opt`)
- A funded Stellar testnet account

**Step 1: Clone and install dependencies**
```bash
cd zk-remittance-stellar
npm install        # installs circomlib, snarkjs
cd contracts
cargo build --target wasm32v1-none --release
cd ../frontend
npm install
cd ..
```

**Step 2: Compile the circuit and generate proving key**
```bash
# Download Powers of Tau (required once)
wget https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_17.ptau \
  -O circuits/build/pot17_final.ptau

# Compile and setup
cd circuits
circom remittance.circom --r1cs --wasm --sym -o build/
cd build
snarkjs groth16 setup remittance.r1cs pot17_final.ptau remittance_0000.zkey
snarkjs zkey contribute remittance_0000.zkey remittance_final.zkey --name="zk-remittance" -v
snarkjs zkey export verificationkey remittance_final.zkey verification_key.json
cd ../..
```

**Step 3: Generate verification key constants for the contract**
```bash
node scripts/vk_to_rust.js < circuits/build/verification_key.json > contracts/src/vk_constants.rs
```

**Step 4: Deploy contract**
```bash
stellar contract upload --network testnet --source alice \
  --wasm contracts/target/wasm32v1-none/release/zk_remittance.wasm

CONTRACT_ID=$(stellar contract deploy --network testnet --source alice \
  --wasm contracts/target/wasm32v1-none/release/zk_remittance.wasm)

stellar contract invoke --network testnet --source alice --id $CONTRACT_ID -- \
  initialize \
  --admin $(stellar keys address alice) \
  --merkle_root 0x0000000000000000000000000000000000000000000000000000000000000000 \
  --token CAQAAONVIWQIYEGRMEI2PMT4K3WGI2GS77YY2R3BHWCO7VSPUQJBHM6H

echo $CONTRACT_ID > .contract_id
```

**Step 5: Run the frontend**
```bash
cd frontend
cp .env.example .env
# Edit .env with your deployed contract ID
npm run dev
```

**Step 6: Generate a test proof**
```bash
node scripts/generate_proof.js
```

### Contract Address (Testnet)

```
Contract ID: <deployed contract ID>
Network: Stellar Testnet (https://soroban-testnet.stellar.org)
```

### Production-Ready vs. Hackathon Shortcuts

| Component | Status | Notes |
|-----------|--------|-------|
| Circom circuit (Merkle proof, range proof, nullifier, recipient binding) | Production-ready | Uses Poseidon hash, depth-20 Merkle tree |
| Groth16 prover (snarkjs) | Production-ready | Standardized, audited |
| Soroban contract (proof verification) | Production-ready | BN254 host functions, constant-time pairing check |
| Merkle tree management | Hackathon | Currently mock — needs a real KYC oracle/hub |
| KYC identity verification | Hackathon | Not implemented — requires a trusted issuer |
| Frontend wallet integration | Hackathon | Uses mock keypair — needs Freighter or Wallet Connect |
| Web Worker for proof generation | Hackathon | Blocks main thread — wrap `groth16.fullProve` in a worker |
| Compliance query privacy | Hackathon | Volume is stored as plain i128 — needs homomorphic approach for production |
| Powers of Tau ceremony | Hackathon | Uses public Hermez PTAU — for production, run a multi-party ceremony |
| Contract ownership | Demo | Uses single admin — production needs multisig or DAO |

### Team

- Your Name Here — [@yourgithub](https://github.com/yourgithub)
- Built for [Stellar Hackathon](https://stellar.org/hackathon)
