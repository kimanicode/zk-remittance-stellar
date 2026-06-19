# ZK Remittance on Stellar

Private cross-border remittance powered by zero-knowledge proofs on Stellar's Soroban platform. Send money under $10,000 with KYC compliance , without revealing your identity, exact amount, or recipient on the public ledger.

### Why ZK

Without ZK, you face an impossible choice: full transparency (every transaction visible on-chain - no privacy) or full anonymity (no compliance - money laundering risk). ZK threads the needle by letting a sender prove "I am KYC-verified, my amount is under $10,000, and I haven't reused this proof before" without revealing _which_ KYC'd user they are, the _exact_ amount, or _who_ the recipient is, beyond a one-way hash. The smart contract verifies one Groth16 proof and releases the funds , it learns only a Merkle root, a nullifier hash, and a recipient hash. Nothing else.

This is genuinely load-bearing ZK: without it, the contract would have to choose between checking KYC status in the clear (no privacy) or skipping the check entirely (no compliance).

### Architecture

```
┌────────────────────────────────────────────────────────────┐
│                      Sender's Browser                       │
│  ┌──────────┐   ┌───────────┐   ┌──────────────────────┐   │
│  │ Circom   │   │  snarkjs  │   │ @noble/curves bls12-381│  │
│  │ Circuit  │──>│ fullProve │──>│ point encoder         │  │
│  │ (WASM)   │   │ (Groth16) │   │                       │  │
│  └──────────┘   └───────────┘   └──────────┬────────────┘  │
│                                              │               │
│                                  @stellar/stellar-sdk         │
│                                  submitTransaction             │
└─────────────────────────────────────────────┼────────────────┘
                                              │
                                              ▼
┌────────────────────────────────────────────────────────────┐
│                     Stellar Testnet                          │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Soroban Smart Contract                   │   │
│  │  ┌──────────┐   ┌──────────┐   ┌────────────────┐    │   │
│  │  │ Groth16  │──>│ Nullifier│──>│ Token Transfer │    │   │
│  │  │ Verify   │   │ Check    │   │ (native XLM)   │    │   │
│  │  │(BLS12-381│   │          │   │                │    │   │
│  │  │ pairing) │   │          │   │                │    │   │
│  │  └──────────┘   └──────────┘   └────────────────┘    │   │
│  │                                                       │   │
│  │  ┌──────────────────────────────────────────────┐    │   │
│  │  │ Compliance Query (volume tally by recipient   │    │   │
│  │  │ hash, returns exceeded/not + proof hash)      │    │   │
│  │  └──────────────────────────────────────────────┘    │   │
│  └──────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────┘
```

**Curve note:** the circuit and trusted setup run on **BLS12-381**, not BN254. We initially built on BN254 since that's what Circom defaults to, but switched after finding that Stellar's own canonical reference (`stellar/soroban-examples/groth16_verifier`) uses BLS12-381's `pairing_check` host function, with no equivalently proven BN254 pattern available. Circom supports compiling to BLS12-381 directly via `circom --prime bls12381`, so the rest of the toolchain (snarkjs, Groth16) is unchanged.

### How to Run Locally

**Prerequisites:**

- Node.js 18+
- Rust with `wasm32v1-none` target (`rustup target add wasm32v1-none`)
- Circom 2.0+ compiled from source (`git clone https://github.com/iden3/circom && cd circom && cargo install --path circom`) - the npm package `circom` is the old, incompatible 1.x line
- Stellar CLI (prebuilt binary recommended: see [stellar/stellar-cli releases](https://github.com/stellar/stellar-cli/releases) - `cargo install stellar-cli` works but requires `pkg-config`, `libssl-dev`, `libdbus-1-dev`, `libudev-dev` system packages on Linux)
- A funded Stellar testnet account (`stellar keys generate alice && stellar keys fund alice --network testnet`)

**Step 1: Install dependencies**

```bash
cd circuits && npm install circomlib && cd ..
cd contracts && cargo build --target wasm32v1-none --release && cd ..
cd frontend && npm install && cd ..
cd tools/vk_encoder && cargo build --release && cd ../..
```

**Step 2: Compile the circuit on BLS12-381**

```bash
cd circuits
mkdir -p build_bls
circom remittance.circom --r1cs --wasm --sym -p bls12381 -o build_bls/
```

**Step 3: Run a trusted setup ceremony**

For a hackathon demo, a local single-contributor ceremony is fine:

```bash
snarkjs powersoftau new bls12381 15 build_bls/pot15_0000.ptau -v
snarkjs powersoftau contribute build_bls/pot15_0000.ptau build_bls/pot15_0001.ptau --name="contribution" -v -e="some entropy"
snarkjs powersoftau prepare phase2 build_bls/pot15_0001.ptau build_bls/pot15_final.ptau -v
snarkjs groth16 setup build_bls/remittance.r1cs build_bls/pot15_final.ptau build_bls/remittance_0000.zkey
snarkjs zkey contribute build_bls/remittance_0000.zkey build_bls/remittance_final.zkey --name="zkey contribution" -e="more entropy"
snarkjs zkey export verificationkey build_bls/remittance_final.zkey build_bls/verification_key.json
cd ..
```

**Step 4: Encode the verification key for the contract**

```bash
./tools/vk_encoder/target/release/vk-encoder vk circuits/build_bls/verification_key.json > circuits/build_bls/vk.hex
node -e "
const hex = require('fs').readFileSync('circuits/build_bls/vk.hex', 'utf8').trim();
const buf = Buffer.from(hex, 'hex');
let pos = 0;
const take = n => { const b = buf.slice(pos, pos+n); pos += n; return b; };
const toArr = b => '[' + Array.from(b).join(', ') + ']';
const alpha = take(96), beta = take(192), gamma = take(192), delta = take(192);
const icCount = take(4).readUInt32BE(0);
const ics = []; for (let i = 0; i < icCount; i++) ics.push(take(96));
let out = '';
out += 'pub const VK_ALPHA: [u8; 96] = ' + toArr(alpha) + ';\n\n';
out += 'pub const VK_BETA: [u8; 192] = ' + toArr(beta) + ';\n\n';
out += 'pub const VK_GAMMA: [u8; 192] = ' + toArr(gamma) + ';\n\n';
out += 'pub const VK_DELTA: [u8; 192] = ' + toArr(delta) + ';\n\n';
out += 'pub const VK_IC: [[u8; 96]; ' + icCount + '] = [\n';
for (const ic of ics) out += '    ' + toArr(ic) + ',\n';
out += '];\n';
require('fs').writeFileSync('contracts/src/vk_constants.rs', out);
"
```

**Step 5: Build and deploy the contract**

```bash
cd contracts && cargo build --target wasm32v1-none --release && cd ..
bash scripts/deploy_contract.sh
# Note: the script's auto-initialize step may fail with "Invalid name" —
# this is a known stellar-cli quirk where it tries to use the contract ID
# as an identity alias. Run initialize manually if so:
stellar contract invoke --network testnet --source alice --id <CONTRACT_ID> \
  -- initialize \
  --admin $(stellar keys address alice) \
  --merkle_root <32_byte_hex_merkle_root> \
  --token <native_xlm_sac_address>
# Get the native XLM SAC address with:
stellar contract id asset --asset native --network testnet
```

**Step 6: Fund the contract**

The contract needs a token balance before it can pay anyone out:

```bash
stellar contract invoke --network testnet --source alice --id <CONTRACT_ID> \
  -- deposit --from $(stellar keys address alice) --amount 100000000
```

**Step 7: Run the frontend**

```bash
cd frontend
cp circuits/build_bls/remittance_js/remittance.wasm public/
cp circuits/build_bls/remittance_final.zkey public/
echo "VITE_CONTRACT_ID=<CONTRACT_ID>" > .env
echo "VITE_ALICE_SECRET=<alice secret key from 'stellar keys show alice'>" >> .env
npm run dev
```

### Contract Address (Testnet)

```
Contract ID: CD4QXKAJXPN2OCZGOKO3O2VKYEO6YJC3WERPBISIT2FJSU6IZYC7W5QO
Network: Stellar Testnet (https://soroban-testnet.stellar.org)
```

### What's Proven On-Chain (not simulated)

We ran the full pipeline against this deployed contract and confirmed, on real testnet transactions:

- A real Groth16 proof, generated from the actual circuit and trusted setup, verified successfully via `env.crypto().bls12_381().pairing_check()` inside the deployed contract
- A real token transfer executed only after that verification passed
- Resubmitting the identical proof a second time was rejected by the nullifier check (`Error(WasmVm, InvalidAction)` - `panic!("nullifier already used")`), confirming replay protection works
- The compliance query returned a correct "exceeded" / "not exceeded" result with a verifiable proof hash, without exposing sender identity

### Production-Ready vs. Hackathon Shortcuts

| Component                                                                | Status           | Notes                                                                                                                                                                                                           |
| ------------------------------------------------------------------------ | ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Circom circuit (Merkle proof, range proof, nullifier, recipient binding) | Production-ready | Poseidon hash, depth-20 Merkle tree, BLS12-381                                                                                                                                                                  |
| Groth16 prover (snarkjs)                                                 | Production-ready | Standard library, runs entirely client-side in-browser                                                                                                                                                          |
| Soroban contract (proof verification)                                    | Production-ready | Uses `soroban_sdk::crypto::bls12_381`, mirrors Stellar's own reference verifier                                                                                                                                 |
| JS BLS12-381 point encoding                                              | Production-ready | Uses audited `@noble/curves`; byte-for-byte verified against our Rust `ark-serialize` encoder                                                                                                                   |
| Merkle tree management                                                   | Hackathon        | Single-leaf tree for demo purposes - needs a real KYC oracle managing the full tree                                                                                                                             |
| KYC identity verification                                                | Hackathon        | Not implemented - requires a trusted issuer producing Merkle leaves                                                                                                                                             |
| Frontend wallet integration                                              | Hackathon        | Uses a hardcoded test keypair - needs Freighter or WalletConnect                                                                                                                                                |
| Web Worker for proof generation                                          | Hackathon        | `groth16.fullProve` currently blocks the main thread for ~10s                                                                                                                                                   |
| Compliance query privacy                                                 | Hackathon        | Volume stored as plain `i128` keyed by recipient hash - a production version would want a more sophisticated scheme (e.g. homomorphic tallying) so even aggregate volume isn't visible to the contract operator |
| Powers of Tau ceremony                                                   | Hackathon        | Single-contributor local ceremony - production needs a real multi-party ceremony                                                                                                                                |
| Contract ownership                                                       | Demo             | Single admin key - production needs multisig or DAO governance                                                                                                                                                  |

### Team

- Kimani Karaba - [github.com/kimanicode](https://github.com/kimanicode)
- Built for the [Stellar Hacks: Real-World ZK](https://dorahacks.io/hackathon/stellar-hacks-zk/detail) hackathon
