pragma circom 2.1.6;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/bitify.circom";
include "circomlib/circuits/comparators.circom";

template SentinelTransfer() {
    signal input sender_commitment_before;
    signal input sender_commitment_after;
    signal input receiver_commitment_before;
    signal input receiver_commitment_after;
    signal input nullifier_hash;

    signal input sender_secret;
    signal input receiver_pubkey_hash;
    signal input sender_balance_before;
    signal input transfer_amount;
    signal input sender_balance_after;
    signal input receiver_balance_before;
    signal input receiver_balance_after;

    component nullifier_hasher = Poseidon(1);
    nullifier_hasher.inputs[0] <== sender_secret;
    nullifier_hasher.out === nullifier_hash;

    component sender_before_hash = Poseidon(2);
    sender_before_hash.inputs[0] <== sender_secret;
    sender_before_hash.inputs[1] <== sender_balance_before;
    sender_before_hash.out === sender_commitment_before;

    component sender_after_hash = Poseidon(2);
    sender_after_hash.inputs[0] <== sender_secret;
    sender_after_hash.inputs[1] <== sender_balance_after;
    sender_after_hash.out === sender_commitment_after;

    component receiver_before_hash = Poseidon(2);
    receiver_before_hash.inputs[0] <== receiver_pubkey_hash;
    receiver_before_hash.inputs[1] <== receiver_balance_before;
    receiver_before_hash.out === receiver_commitment_before;

    component receiver_after_hash = Poseidon(2);
    receiver_after_hash.inputs[0] <== receiver_pubkey_hash;
    receiver_after_hash.inputs[1] <== receiver_balance_after;
    receiver_after_hash.out === receiver_commitment_after;

    sender_balance_after === sender_balance_before - transfer_amount;
    receiver_balance_after === receiver_balance_before + transfer_amount;

    component transfer_bits = Num2Bits(64);
    transfer_bits.in <== transfer_amount;
    component transfer_not_zero = IsZero();
    transfer_not_zero.in <== transfer_amount;
    transfer_not_zero.out === 0;

    component sender_after_bits = Num2Bits(64);
    sender_after_bits.in <== sender_balance_after;

    component receiver_after_bits = Num2Bits(64);
    receiver_after_bits.in <== receiver_balance_after;
}

component main {public [
    sender_commitment_before,
    sender_commitment_after,
    receiver_commitment_before,
    receiver_commitment_after,
    nullifier_hash
]} = SentinelTransfer();
