const { buildPoseidon } = require("circomlibjs");

async function main() {
  const poseidon = await buildPoseidon();
  const F = poseidon.F;

  const address = BigInt("1234567890");
  const path = Array(20).fill(BigInt(0));
  const indices = Array(20).fill(0);

  let current = address;

  for (let i = 0; i < 20; i++) {
    const sibling = path[i];
    let left, right;
    if (indices[i] === 0) {
      left = current;
      right = sibling;
    } else {
      left = sibling;
      right = current;
    }
    current = F.toObject(poseidon([left, right]));
  }

  console.log("merkle_root:", current.toString());
}

main().catch(console.error);
