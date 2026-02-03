pragma circom 2.0.0;

template Sentinel() {
    signal input secret;
    signal output hash;
    hash <== secret * secret;
}

component main = Sentinel();