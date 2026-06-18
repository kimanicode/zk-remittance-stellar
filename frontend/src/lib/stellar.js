import {
  Contract,
  Keypair,
  Networks,
  SorobanRpc,
  TransactionBuilder,
  nativeToScVal,
  xdr,
} from "@stellar/stellar-sdk";

const SERVER_URL =
  import.meta.env.VITE_SOROBAN_RPC_URL || "https://soroban-testnet.stellar.org";
const CONTRACT_ID = import.meta.env.VITE_CONTRACT_ID || "";
const NETWORK_PASSPHRASE = Networks.TESTNET;

const server = new SorobanRpc.Server(SERVER_URL);
const contract = new Contract(CONTRACT_ID);

function hexToBytes(hex) {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(clean.substr(i * 2, 2), 16);
  }
  return bytes;
}

function decimalToU256ScVal(decimalStr) {
  // Build a u256 ScVal from a decimal string using BigInt -> 4x u64 limbs
  const n = BigInt(decimalStr);
  const mask64 = (1n << 64n) - 1n;
  const hiHi = (n >> 192n) & mask64;
  const hiLo = (n >> 128n) & mask64;
  const loHi = (n >> 64n) & mask64;
  const loLo = n & mask64;
  return xdr.ScVal.scvU256(
    new xdr.UInt256Parts({
      hiHi: xdr.Uint64.fromString(hiHi.toString()),
      hiLo: xdr.Uint64.fromString(hiLo.toString()),
      loHi: xdr.Uint64.fromString(loHi.toString()),
      loLo: xdr.Uint64.fromString(loLo.toString()),
    }),
  );
}

export async function submitProof({
  senderKeypair,
  proofAHex,
  proofBHex,
  proofCHex,
  publicSignalsDecimal, // array of 3 decimal strings: [nullifier, recipientHash, merkleRoot]
  merkleRootHex,
  nullifierHashHex,
  recipientHashHex,
  recipient,
  amount,
}) {
  const keypair =
    typeof senderKeypair === "string"
      ? Keypair.fromSecret(senderKeypair)
      : senderKeypair;
  const account = await server.getAccount(keypair.publicKey());

  const proofA = xdr.ScVal.scvBytes(hexToBytes(proofAHex));
  const proofB = xdr.ScVal.scvBytes(hexToBytes(proofBHex));
  const proofC = xdr.ScVal.scvBytes(hexToBytes(proofCHex));

  const publicSignals = xdr.ScVal.scvVec(
    publicSignalsDecimal.map((d) => decimalToU256ScVal(d)),
  );

  const merkleRootBytes = xdr.ScVal.scvBytes(hexToBytes(merkleRootHex));
  const nullifierHashBytes = xdr.ScVal.scvBytes(hexToBytes(nullifierHashHex));
  const recipientHashBytes = xdr.ScVal.scvBytes(hexToBytes(recipientHashHex));

  const tx = new TransactionBuilder(account, {
    fee: "1000000",
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(
      contract.call(
        "send",
        nativeToScVal(keypair.publicKey(), { type: "address" }),
        proofA,
        proofB,
        proofC,
        publicSignals,
        merkleRootBytes,
        nullifierHashBytes,
        recipientHashBytes,
        nativeToScVal(recipient, { type: "address" }),
        nativeToScVal(amount, { type: "i128" }),
      ),
    )
    .setTimeout(30)
    .build();

  const preparedTx = await server.prepareTransaction(tx);
  preparedTx.sign(keypair);
  const result = await server.sendTransaction(preparedTx);
  return result;
}

export async function queryCompliance({ addressHashHex, threshold }) {
  const contractAccount = await server
    .getAccount(Keypair.random().publicKey())
    .catch(() => null);

  // Use a throwaway funded account isn't ideal for sim; instead use simulate with a dummy source
  // For simplicity in the demo, reuse alice's account for simulation context
  const aliceSecret = import.meta.env.VITE_ALICE_SECRET;
  const aliceKeypair = Keypair.fromSecret(aliceSecret);
  const account = await server.getAccount(aliceKeypair.publicKey());

  const tx = new TransactionBuilder(account, {
    fee: "1000000",
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(
      contract.call(
        "compliance_query",
        xdr.ScVal.scvBytes(hexToBytes(addressHashHex)),
        nativeToScVal(threshold, { type: "i128" }),
      ),
    )
    .setTimeout(30)
    .build();

  const result = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(result)) {
    throw new Error(result.error);
  }

  const retval = result.result?.retval;
  if (!retval) return { exceeded: false, nullifierHash: "" };

  const parts = retval.vec();
  const exceeded = parts[0].b();
  const nullifierBytes = parts[1].bytes();
  const nullifierHash = Array.from(nullifierBytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");

  return { exceeded, nullifierHash };
}
