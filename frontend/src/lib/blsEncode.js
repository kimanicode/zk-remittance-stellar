import { bls12_381 } from "@noble/curves/bls12-381.js";

export function encodeG1(pi) {
  const x = BigInt(pi[0]);
  const y = BigInt(pi[1]);
  const point = bls12_381.G1.Point.fromAffine({ x, y });
  point.assertValidity();
  return point.toBytes(false); // uncompressed, 96 bytes
}

export function encodeG2(pi) {
  const x = { c0: BigInt(pi[0][0]), c1: BigInt(pi[0][1]) };
  const y = { c0: BigInt(pi[1][0]), c1: BigInt(pi[1][1]) };
  const point = bls12_381.G2.Point.fromAffine({ x, y });
  point.assertValidity();
  return point.toBytes(false); // uncompressed, 192 bytes
}

export function bytesToHex(bytes) {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

export function decimalToHex32(decimalStr) {
  const n = BigInt(decimalStr);
  return n.toString(16).padStart(64, "0");
}
