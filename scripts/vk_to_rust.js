/**
 * Converts snarkjs verification_key.json to Rust Soroban-compatible constants.
 *
 * Usage:
 *   node scripts/vk_to_rust.js < circuits/build/verification_key.json
 */
const fs = require("fs");

function bigIntToLeBytes(bigint) {
  const hex = bigint.toString(16).padStart(64, "0");
  const bytes = [];
  for (let i = 0; i < 32; i++) {
    const byte = parseInt(hex.substring(i * 2, i * 2 + 2), 16);
    bytes.push(byte);
  }
  return bytes;
}

function formatG1Bytes(p) {
  const x = BigInt(p[0]);
  const y = BigInt(p[1]);
  const xBytes = bigIntToLeBytes(x);
  const yBytes = bigIntToLeBytes(y);
  return `([${xBytes.join(", ")}], [${yBytes.join(", ")}])`;
}

function formatG2Bytes(p) {
  // snarkjs G2 points: [[x_re, x_im], [y_re, y_im], [z_re, z_im]]
  // p[0] = x, p[1] = y (we ignore p[2] which is z=1 in affine)
  const xRe = BigInt(p[0][0]);
  const xIm = BigInt(p[0][1]);
  const yRe = BigInt(p[1][0]);
  const yIm = BigInt(p[1][1]);

  const xReBytes = bigIntToLeBytes(xRe);
  const xImBytes = bigIntToLeBytes(xIm);
  const yReBytes = bigIntToLeBytes(yRe);
  const yImBytes = bigIntToLeBytes(yIm);

  return `(([${xReBytes.join(", ")}], [${xImBytes.join(", ")}]), ([${yReBytes.join(", ")}], [${yImBytes.join(", ")}]))`;
}

function main() {
  const raw = fs.readFileSync(0, "utf-8");
  const vk = JSON.parse(raw);
  const name = process.argv[2] || "VERIFICATION_KEY_BYTES";

  console.log(`// Auto-generated from verification_key.json on ${new Date().toISOString()}
// Do not edit manually.
// Regenerate with: node scripts/vk_to_rust.js < circuits/build/verification_key.json

use soroban_sdk::{Env, Vec, BytesN, vec};
use crate::groth16::VerificationKey;

`);

  // Alpha1
  const a1 = formatG1Bytes(vk.vk_alpha_1);
  console.log(`pub const ALPHA1: ([u8; 32], [u8; 32]) = ${a1};`);
  console.log("");

  // Beta2
  const b2 = formatG2Bytes(vk.vk_beta_2);
  console.log(
    `pub const BETA2: (([u8; 32], [u8; 32]), ([u8; 32], [u8; 32])) = ${b2};`,
  );
  console.log("");

  // Gamma2
  const g2 = formatG2Bytes(vk.vk_gamma_2);
  console.log(
    `pub const GAMMA2: (([u8; 32], [u8; 32]), ([u8; 32], [u8; 32])) = ${g2};`,
  );
  console.log("");

  // Delta2
  const d2 = formatG2Bytes(vk.vk_delta_2);
  console.log(
    `pub const DELTA2: (([u8; 32], [u8; 32]), ([u8; 32], [u8; 32])) = ${d2};`,
  );
  console.log("");

  // IC
  console.log("pub const IC: &[([u8; 32], [u8; 32])] = &[");
  for (const p of vk.IC) {
    console.log(`    ${formatG1Bytes(p)},`);
  }
  console.log("];");
  console.log("");

  // Helper function
  console.log(`pub fn get_verification_key(env: &Env) -> VerificationKey {
    VerificationKey {
        alpha1: (
            BytesN::from_array(env, &ALPHA1.0),
            BytesN::from_array(env, &ALPHA1.1),
        ),
        beta2: (
            (
                BytesN::from_array(env, &BETA2.0 .0),
                BytesN::from_array(env, &BETA2.0 .1),
            ),
            (
                BytesN::from_array(env, &BETA2.1 .0),
                BytesN::from_array(env, &BETA2.1 .1),
            ),
        ),
        gamma2: (
            (
                BytesN::from_array(env, &GAMMA2.0 .0),
                BytesN::from_array(env, &GAMMA2.0 .1),
            ),
            (
                BytesN::from_array(env, &GAMMA2.1 .0),
                BytesN::from_array(env, &GAMMA2.1 .1),
            ),
        ),
        delta2: (
            (
                BytesN::from_array(env, &DELTA2.0 .0),
                BytesN::from_array(env, &DELTA2.0 .1),
            ),
            (
                BytesN::from_array(env, &DELTA2.1 .0),
                BytesN::from_array(env, &DELTA2.1 .1),
            ),
        ),
        ic: {
            let mut v = vec![env];
            for i in 0..IC.len() {
                v.push_back((
                    BytesN::from_array(env, &IC[i].0),
                    BytesN::from_array(env, &IC[i].1),
                ));
            }
            v
        },
    }
}`);
}

main();
