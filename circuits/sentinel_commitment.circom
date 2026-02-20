pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/bitify.circom";
include "circomlib/circuits/comparators.circom";

template SentinelCommitment() {
    signal input commitment;
    signal input nullifier_hash;

    signal input secret;
    signal input amount;

    component commitment_hasher = Poseidon(2);
    commitment_hasher.inputs[0] <== secret;
    commitment_hasher.inputs[1] <== amount;
    commitment_hasher.out === commitment;

    component nullifier_hasher = Poseidon(1);
    nullifier_hasher.inputs[0] <== secret;
    nullifier_hasher.out === nullifier_hash;

    component amount_bits = Num2Bits(64);
    amount_bits.in <== amount;

    signal amount_is_zero;
    component is_zero = IsZero();
    is_zero.in <== amount;
    is_zero.out === 0;
}

component main {public [commitment, nullifier_hash]} = SentinelCommitment();
