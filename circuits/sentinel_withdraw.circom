pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/bitify.circom";
include "circomlib/circuits/comparators.circom";

template SentinelWithdraw() {
    signal input balance_commitment_before;
    signal input balance_commitment_after;
    signal input withdraw_amount;
    signal input nullifier_hash;

    signal input secret;
    signal input balance_before;
    signal input balance_after;

    component nullifier_hasher = Poseidon(1);
    nullifier_hasher.inputs[0] <== secret;
    nullifier_hasher.out === nullifier_hash;

    component before_hash = Poseidon(2);
    before_hash.inputs[0] <== secret;
    before_hash.inputs[1] <== balance_before;
    before_hash.out === balance_commitment_before;

    component after_hash = Poseidon(2);
    after_hash.inputs[0] <== secret;
    after_hash.inputs[1] <== balance_after;
    after_hash.out === balance_commitment_after;

    balance_after === balance_before - withdraw_amount;

    component withdraw_bits = Num2Bits(64);
    withdraw_bits.in <== withdraw_amount;
    component withdraw_not_zero = IsZero();
    withdraw_not_zero.in <== withdraw_amount;
    withdraw_not_zero.out === 0;

    component after_bits = Num2Bits(64);
    after_bits.in <== balance_after;
}

component main {public [
    balance_commitment_before,
    balance_commitment_after,
    withdraw_amount,
    nullifier_hash
]} = SentinelWithdraw();
