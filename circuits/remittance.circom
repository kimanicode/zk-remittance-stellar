pragma circom 2.0.0;

include "node_modules/circomlib/circuits/poseidon.circom";
include "node_modules/circomlib/circuits/comparators.circom";
include "node_modules/circomlib/circuits/mux1.circom";

template MerkleProof(depth) {
    signal input leaf;
    signal input root;
    signal input path_indices[depth];
    signal input siblings[depth];

    component selectors[depth];
    component hashers[depth];
    signal computed[depth+1];

    computed[0] <== leaf;

    for (var i = 0; i < depth; i++) {
        selectors[i] = MultiMux1(2);
        selectors[i].c[0][0] <== computed[i];
        selectors[i].c[0][1] <== siblings[i];
        selectors[i].c[1][0] <== siblings[i];
        selectors[i].c[1][1] <== computed[i];
        selectors[i].s <== path_indices[i];

        hashers[i] = Poseidon(2);
        hashers[i].inputs[0] <== selectors[i].out[0];
        hashers[i].inputs[1] <== selectors[i].out[1];

        computed[i+1] <== hashers[i].out;
    }

    root === computed[depth];
}

template RemittanceProof() {
    signal input merkle_root;

    signal input address;
    signal input merkle_path_indices[20];
    signal input merkle_path[20];
    signal input amount;
    signal input nonce;
    signal input recipient_address;

    signal output nullifier_hash;
    signal output recipient_address_hash;

    component merkle = MerkleProof(20);
    merkle.leaf <== address;
    merkle.root <== merkle_root;
    for (var i = 0; i < 20; i++) {
        merkle.path_indices[i] <== merkle_path_indices[i];
        merkle.siblings[i] <== merkle_path[i];
    }

    component lt = LessThan(20);
    lt.in[0] <== amount;
    lt.in[1] <== 1000000;
    lt.out === 1;

    component nullifier = Poseidon(2);
    nullifier.inputs[0] <== address;
    nullifier.inputs[1] <== nonce;
    nullifier_hash <== nullifier.out;

    component recipient = Poseidon(1);
    recipient.inputs[0] <== recipient_address;
    recipient_address_hash <== recipient.out;
}

component main { public [merkle_root] } = RemittanceProof();