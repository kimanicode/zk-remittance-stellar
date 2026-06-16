const snarkjs = require("snarkjs");
const fs = require("fs");
const path = require("path");

async function generateProof(input) {
  const wasmPath = path.join(
    __dirname,
    "..",
    "circuits",
    "build",
    "remittance_js",
    "remittance.wasm",
  );
  const zkeyPath = path.join(
    __dirname,
    "..",
    "circuits",
    "build",
    "remittance_final.zkey",
  );

  const { proof, publicSignals } = await snarkjs.groth16.fullProve(
    input,
    wasmPath,
    zkeyPath,
  );

  const calldata = await snarkjs.groth16.exportSolidityCallData(
    proof,
    publicSignals,
  );

  return { proof, publicSignals, calldata };
}

async function main() {
  const input = {
    merkle_root:
      "21366341404617559109078536841419919559345075898020461913054443477245838208360",
    address: "1234567890",
    merkle_path: Array(20).fill("0"),
    merkle_path_indices: Array(20).fill("0"),
    amount: "500000",
    nonce: "9999",
    recipient_address: "9876543210",
  };

  console.log("Generating proof with input:", JSON.stringify(input, null, 2));

  try {
    const result = await generateProof(input);

    const buildDir = path.join(__dirname, "..", "circuits", "build");
    fs.writeFileSync(
      path.join(buildDir, "proof.json"),
      JSON.stringify(result.proof, null, 2),
    );
    fs.writeFileSync(
      path.join(buildDir, "public.json"),
      JSON.stringify(result.publicSignals, null, 2),
    );

    console.log(
      "\n Public signals:",
      JSON.stringify(result.publicSignals, null, 2),
    );
    console.log("\n Proof written to circuits/build/proof.json");
    console.log(" Public signals written to circuits/build/public.json");

    const parsed = JSON.parse("[" + result.calldata + "]");
    console.log(
      "\n Calldata (first 100 chars):",
      result.calldata.substring(0, 100) + "...",
    );

    const localVerify = await snarkjs.groth16.verify(
      JSON.parse(fs.readFileSync(path.join(buildDir, "verification_key.json"))),
      result.publicSignals,
      result.proof,
    );
    console.log("\n Local verification:", localVerify ? "PASSED " : "FAILED ");
  } catch (err) {
    console.error("Error generating proof:", err);
    process.exit(1);
  }
}

main();
