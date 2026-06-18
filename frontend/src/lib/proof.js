import { groth16 } from "snarkjs";
import { StrKey } from "@stellar/stellar-sdk";
import { encodeG1, encodeG2, bytesToHex, decimalToHex32 } from "./blsEncode.js";

function addressToField(address) {
  try {
    const raw = StrKey.decodeEd25519PublicKey(address);
    const hex = Array.from(raw.slice(0, 31))
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
    return BigInt("0x" + hex).toString();
  } catch (e) {
    throw new Error(`Cannot convert ${address} to a BigInt: ${e.message}`);
  }
}

export async function generateProof({
  senderAddress,
  recipientAddress,
  amount,
  nonce,
  merklePath,
  merklePathIndices,
  merkleRoot,
}) {
  if (!merklePath) merklePath = Array(20).fill("0");
  if (!merklePathIndices) merklePathIndices = Array(20).fill("0");
  if (!nonce) nonce = Math.floor(Math.random() * 1000000).toString();

  const input = {
    merkle_root:
      merkleRoot ||
      "7846114547599950979977548495961514500109843146722585183135239779897529274437",
    address: addressToField(senderAddress),
    merkle_path: merklePath,
    merkle_path_indices: merklePathIndices,
    amount: Math.round(amount * 100).toString(),
    nonce: nonce.toString(),
    recipient_address: addressToField(recipientAddress),
  };
  console.log("DEBUG address field:", input.address);
  console.log("DEBUG merkle_root:", input.merkle_root);

  const { proof, publicSignals } = await groth16.fullProve(
    input,
    "/remittance.wasm",
    "/remittance_final.zkey",
  );

  // publicSignals order: [nullifier_hash, recipient_hash, merkle_root]
  const proofAHex = bytesToHex(encodeG1(proof.pi_a));
  const proofBHex = bytesToHex(encodeG2(proof.pi_b));
  const proofCHex = bytesToHex(encodeG1(proof.pi_c));

  const nullifierHashHex = decimalToHex32(publicSignals[0]);
  const recipientHashHex = decimalToHex32(publicSignals[1]);
  const merkleRootHex = decimalToHex32(publicSignals[2]);

  return {
    proofAHex,
    proofBHex,
    proofCHex,
    publicSignalsDecimal: publicSignals.map((s) => s.toString()),
    merkleRootHex,
    nullifierHashHex,
    recipientHashHex,
    nonce,
  };
}
