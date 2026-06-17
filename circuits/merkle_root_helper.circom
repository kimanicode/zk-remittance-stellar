pragma circom 2.0.0;
include "node_modules/circomlib/circuits/poseidon.circom";

template ComputeRoot(depth) {
    signal input leaf;
    signal output root;
    component hashers[depth];
    signal computed[depth+1];
    computed[0] <== leaf;
    for (var i = 0; i < depth; i++) {
        hashers[i] = Poseidon(2);
        hashers[i].inputs[0] <== computed[i];
        hashers[i].inputs[1] <== 0;
        computed[i+1] <== hashers[i].out;
    }
    root <== computed[depth];
}
component main = ComputeRoot(20);
