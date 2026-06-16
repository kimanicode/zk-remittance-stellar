import {
  Contract,
  Networks,
  Keypair,
  SorobanRpc,
  TransactionBuilder,
  nativeToScVal,
  xdr,
  Address as StellarAddress,
} from "@stellar/stellar-sdk";

function decimalToBytes32(decimalStr) {
  const hex = BigInt(decimalStr).toString(16).padStart(64, "0");
  const bytes = new Uint8Array(32);
  for (let i = 0; i < 32; i++) {
    bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes.reverse();
}

const SERVER_URL =
  import.meta.env.VITE_SOROBAN_RPC_URL || "https://soroban-testnet.stellar.org";
const CONTRACT_ID = import.meta.env.VITE_CONTRACT_ID || "";
const NETWORK_PASSPHRASE = Networks.TESTNET;

const server = new SorobanRpc.Server(SERVER_URL);
const contract = new Contract(CONTRACT_ID);

function scValFromProofPoint(x, y) {
  return nativeToScVal(
    { x: x, y: y },
    { type: "struct", fields: { x: "bytes32", y: "bytes32" } },
  );
}

export async function submitProof({
  senderKeypair,
  proof,
  publicSignals,
  recipient,
  amount,
}) {
  const keypair =
    typeof senderKeypair === "string"
      ? Keypair.fromSecret(senderKeypair)
      : senderKeypair;
  const account = await server.getAccount(keypair.publicKey());

  const proofA = xdr.ScVal.scvVec([
    xdr.ScVal.scvBytes(decimalToBytes32(proof.pi_a[0])),
    xdr.ScVal.scvBytes(decimalToBytes32(proof.pi_a[1])),
  ]);
  const proofB = xdr.ScVal.scvVec([
    xdr.ScVal.scvVec([
      xdr.ScVal.scvBytes(decimalToBytes32(proof.pi_b[0][0])),
      xdr.ScVal.scvBytes(decimalToBytes32(proof.pi_b[0][1])),
    ]),
    xdr.ScVal.scvVec([
      xdr.ScVal.scvBytes(decimalToBytes32(proof.pi_b[1][0])),
      xdr.ScVal.scvBytes(decimalToBytes32(proof.pi_b[1][1])),
    ]),
  ]);
  const proofC = xdr.ScVal.scvVec([
    xdr.ScVal.scvBytes(decimalToBytes32(proof.pi_c[0])),
    xdr.ScVal.scvBytes(decimalToBytes32(proof.pi_c[1])),
  ]);
  const pubSignals = xdr.ScVal.scvVec(
    publicSignals.map((s) => xdr.ScVal.scvBytes(decimalToBytes32(s))),
  );

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
        pubSignals,
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

export async function queryCompliance({ addressHash, threshold }) {
  const account = await server.getAccount(CONTRACT_ID);

  const tx = new TransactionBuilder(account, {
    fee: "1000000",
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(
      contract.call(
        "compliance_query",
        nativeToScVal(addressHash, { type: "bytes32" }),
        nativeToScVal(threshold, { type: "i128" }),
      ),
    )
    .setTimeout(30)
    .build();

  const result = await server.simulateTransaction(tx);

  if (result.error) {
    throw new Error(result.error);
  }

  // Parse the simulation result
  const returnValue = result.result?.retval;
  if (returnValue) {
    return {
      exceeded: returnValue[0] === true,
      nullifierHash: returnValue[1]?.toString() || "",
    };
  }

  return { exceeded: false, nullifierHash: "" };
}
