import { groth16 } from "snarkjs";
import { StrKey } from "@stellar/stellar-sdk";

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
  if (!merklePath) {
    merklePath = Array(20).fill("0");
  }
  if (!merklePathIndices) {
    merklePathIndices = Array(20).fill("0");
  }
  if (!nonce) {
    nonce = Math.floor(Math.random() * 1000000).toString();
  }

  const input = {
    merkle_root:
      merkleRoot ||
      "21366341404617559109078536841419919559345075898020461913054443477245838208360",
    address: addressToField(senderAddress),
    merkle_path: merklePath,
    merkle_path_indices: merklePathIndices,
    amount: Math.round(amount * 100).toString(),
    nonce: nonce.toString(),
    recipient_address: addressToField(recipientAddress),
  };

  const { proof, publicSignals } = await groth16.fullProve(
    input,
    "/remittance.wasm",
    "/remittance_final.zkey",
  );

  return { proof, publicSignals, nonce };
}

export function formatProofForContract(proof, publicSignals) {
  // Convert proof to Soroban contract format
  const proofA = [proof.pi_a[0], proof.pi_a[1]];
  const proofB = [
    [proof.pi_b[0][0], proof.pi_b[0][1]],
    [proof.pi_b[1][0], proof.pi_b[1][1]],
  ];
  const proofC = [proof.pi_c[0], proof.pi_c[1]];
  const pubSignals = publicSignals.map((s) => s.toString());

  return { proofA, proofB, proofC, pubSignals };
}

export function bigIntToBytes32BE(bigint) {
  const hex = bigint.toString(16).padStart(64, "0");
  return "0x" + hex;
}
